// Copyright 2025 Colin Finck <colin@reactos.org>
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::cmp::Ordering;
use std::collections::HashMap;

use dlv_list::VecList;
use indexmap::IndexSet;

use crate::steps::load_from_registry::RegistryInfo;
use crate::NtLoadOrderEntry;

use super::move_matching_elements_to_front;

pub fn sort_by_tag_and_group(registry_info: RegistryInfo) -> VecList<NtLoadOrderEntry> {
    let mut entries = registry_info
        .entries
        .into_iter()
        .rev()
        .collect::<VecList<NtLoadOrderEntry>>();

    sort_list_by_tag(&mut entries, registry_info.groups);
    sort_list_by_group(&mut entries, registry_info.service_group_order);

    entries
}

fn sort_list_by_tag(
    entries: &mut VecList<NtLoadOrderEntry>,
    groups: HashMap<String, IndexSet<u32>>,
) {
    let start = entries.front_index();
    let end = entries.back_index();

    if start == end {
        return;
    }

    let start = start.unwrap();
    let end = end.unwrap();
    let mut current = start;

    while current != end {
        let next = entries.get_next_index(current).unwrap();
        let current_entry = entries.get(current).unwrap();
        let next_entry = entries.get(next).unwrap();

        if compare_tags(current_entry, next_entry, &groups) == Ordering::Greater {
            // This entry needs to move.
            // Find the right spot to insert it among the entries before this one.
            let mut target = entries.front_index().unwrap();

            while target != current {
                let target_entry = entries.get(target).unwrap();

                match compare_tags(next_entry, target_entry, &groups) {
                    Ordering::Less | Ordering::Equal => break,
                    Ordering::Greater => target = entries.get_next_index(target).unwrap(),
                }
            }

            entries.move_before(next, target);
        }

        current = next;
    }
}

fn sort_list_by_group(entries: &mut VecList<NtLoadOrderEntry>, service_group_order: Vec<String>) {
    // The first moved element in the following loop moves down every time new elements are pushed to the front.
    // But it also marks the separation line between sorted and still unsorted elements.
    // So when going from last to first element, we can stop searching as soon as we end up here.
    let mut first_moved = None;

    for group_name in service_group_order.iter().rev() {
        move_matching_elements_to_front(entries, &mut first_moved, |entry| {
            let Some(entry_group) = &entry.group else {
                return false;
            };
            entry_group.search_key.eq_ignore_ascii_case(group_name)
        });
    }
}

fn compare_tags(
    a: &NtLoadOrderEntry,
    b: &NtLoadOrderEntry,
    groups: &HashMap<String, IndexSet<u32>>,
) -> Ordering {
    let (a_tag, b_tag) = match (a.tag, b.tag) {
        (None, None) => return Ordering::Equal,
        (None, Some(_)) => return Ordering::Greater,
        (Some(_), None) => return Ordering::Less,
        (Some(a_tag), Some(b_tag)) => (a_tag, b_tag),
    };

    let (a_group, b_group) = match (&a.group, &b.group) {
        (None, None) => return Ordering::Equal,
        (None, Some(_)) => return Ordering::Greater,
        (Some(_), None) => return Ordering::Less,
        (Some(a_group), Some(b_group)) => (a_group, b_group),
    };

    let a_index = get_tag_index(a_tag, &a_group.search_key, groups);
    let b_index = get_tag_index(b_tag, &b_group.search_key, groups);

    a_index.cmp(&b_index)
}

fn get_tag_index(
    tag: u32,
    group_search_key: &str,
    groups: &HashMap<String, IndexSet<u32>>,
) -> usize {
    if let Some(set) = groups.get(group_search_key) {
        if let Some(index) = set.get_index_of(&tag) {
            // Convert to a 1-based index.
            // The exact integer value becomes important once elements with entries in GroupOrderList are mixed
            // with elements, where the tag is used as an index (see below).
            index + 1
        } else {
            // Second to last
            0xffff_fffe
        }
    } else {
        // The "Core" group has tags, but no entry in GroupOrderList.
        // Treat the tag as the index.
        tag as usize
    }
}
