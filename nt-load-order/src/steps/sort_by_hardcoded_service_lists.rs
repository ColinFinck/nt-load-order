// Copyright 2025 Colin Finck <colin@reactos.org>
// SPDX-License-Identifier: MIT OR Apache-2.0

use dlv_list::VecList;

use crate::steps::move_matching_elements_to_front;
use crate::NtLoadOrderEntry;

pub fn sort_by_hardcoded_service_lists(entries: &mut VecList<NtLoadOrderEntry>) {
    // The Windows bootloader hardcodes some service lists.
    // Services on these lists are loaded first, irrespective of the group/tag sorting.
    const HARDCODED_LISTS: &[(&str, &[&str])] = &[
        (
            "Core Driver Services",
            &[
                "system32\\drivers\\verifierext.sys",
                "system32\\drivers\\wdf01000.sys",
                "system32\\drivers\\acpiex.sys",
                "system32\\drivers\\cng.sys",
                "system32\\drivers\\mssecflt.sys",
                "system32\\drivers\\sgrmagent.sys",
                "system32\\drivers\\lxss.sys",
                "system32\\drivers\\palcore.sys",
            ],
        ),
        (
            "TPM Core Driver Services",
            &[
                "system32\\drivers\\acpisim.sys",
                "system32\\drivers\\acpi.sys",
            ],
        ),
    ];

    let mut first_moved = None;

    // We move elements to the front, so iterate backwards to retain the order above.
    for (list_name, list_image_paths) in HARDCODED_LISTS.iter().rev() {
        for list_image_path in list_image_paths.iter().rev() {
            move_matching_elements_to_front(entries, &mut first_moved, |entry| {
                let matches = entry.image_path.eq_ignore_ascii_case(list_image_path);

                if matches {
                    entry.reason = format!(
                        "{}, loaded earlier due to hardcoded \"{list_name}\" list",
                        entry.reason
                    );
                }

                matches
            });
        }
    }
}
