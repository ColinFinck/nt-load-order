// Copyright 2025 Colin Finck <colin@reactos.org>
// SPDX-License-Identifier: MIT OR Apache-2.0

use dlv_list::VecList;

use crate::steps::move_matching_elements_to_front;
use crate::NtLoadOrderEntry;

pub fn sort_by_hardcoded_groups(entries: &mut VecList<NtLoadOrderEntry>) {
    // The Windows bootloader also hardcodes some groups and puts them first,
    // irrespective of the ServiceGroupOrder.
    const HARDCODED_GROUPS: &[&str] = &[
        "Early-Launch",
        "Core Platform Extensions",
        "Core Security Extensions",
    ];

    let mut first_moved = None;

    // We move elements to the front, so iterate backwards to retain the order above.
    for group_name in HARDCODED_GROUPS.iter().rev() {
        let group_search_key = group_name.to_ascii_lowercase();

        move_matching_elements_to_front(entries, &mut first_moved, |entry| {
            let Some(entry_group) = &entry.group else {
                return false;
            };

            if entry_group.search_key == group_search_key {
                entry.reason = format!(
                    "{}, loaded earlier due to hardcoded \"{group_name}\" group",
                    entry.reason
                );
                true
            } else {
                false
            }
        });
    }
}
