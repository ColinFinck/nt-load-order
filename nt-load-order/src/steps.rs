// Copyright 2025 Colin Finck <colin@reactos.org>
// SPDX-License-Identifier: MIT OR Apache-2.0

mod add_imports;
mod add_kernel_binaries;
mod load_from_registry;
mod sort_by_hardcoded_groups;
mod sort_by_hardcoded_service_lists;
mod sort_by_tag_and_group;

pub use add_imports::add_imports;
pub use add_kernel_binaries::{add_basic_kernel_binaries, add_kernel_binary};
pub use load_from_registry::load_from_registry;
pub use sort_by_hardcoded_groups::sort_by_hardcoded_groups;
pub use sort_by_hardcoded_service_lists::sort_by_hardcoded_service_lists;
pub use sort_by_tag_and_group::sort_by_tag_and_group;

use dlv_list::{Index, VecList};

use crate::NtLoadOrderEntry;

/// Helper function to iterate through `entries` from back to front,
/// look for elements specified by the predicate,
/// and move them to the front of the `entries` list.
///
/// Iteration is stopped when there are no more elements
/// or when `first_moved` is reached.
///
/// If `first_moved` is `None`, it will be set to the first moved element.
pub(crate) fn move_matching_elements_to_front<F>(
    entries: &mut VecList<NtLoadOrderEntry>,
    first_moved: &mut Option<Index<NtLoadOrderEntry>>,
    mut f: F,
) where
    F: FnMut(&mut NtLoadOrderEntry) -> bool,
{
    let mut current = entries.back_index().unwrap();

    // Push group members to front.
    loop {
        let previous = entries.get_previous_index(current);
        let current_entry = entries.get_mut(current).unwrap();

        if f(current_entry) {
            let front = entries.front_index().unwrap();

            // move_before panics if both parameters are the same.
            if current != front {
                entries.move_before(current, front);
            }

            if first_moved.is_none() {
                *first_moved = Some(current);
            }
        }

        if previous == *first_moved {
            // Don't re-sort what has already been sorted (or we would end up in an infinite loop).
            break;
        }

        match previous {
            Some(previous) => current = previous,
            None => break,
        }
    }
}
