use dlv_list::{Index, VecList};

use crate::NtLoadOrderEntry;

/// Adds "ntoskrnl.exe" and "hal.dll".
/// Returns the [`Index`] of the last added binary.
///
/// You are supposed to then add any KD driver (e.g. "kdcom.dll") and the mcupdate
/// library (e.g. "mcupdate_AuthenticAMD.dll") yourself.
pub fn add_basic_kernel_binaries(
    entries: &mut VecList<NtLoadOrderEntry>,
) -> Index<NtLoadOrderEntry> {
    let ntoskrnl = entries.push_front(NtLoadOrderEntry {
        name: "ntoskrnl".to_string(),
        image_path: "System32\\ntoskrnl.exe".to_string(),
        group: None,
        tag: None,
        reason: "Kernel binary".to_string(),
        is_kernel_binary: true,
    });
    add_kernel_binary(
        entries,
        ntoskrnl,
        "hal".to_string(),
        "System32\\hal.dll".to_string(),
    )
}

pub fn add_kernel_binary(
    entries: &mut VecList<NtLoadOrderEntry>,
    after: Index<NtLoadOrderEntry>,
    name: String,
    image_path: String,
) -> Index<NtLoadOrderEntry> {
    entries.insert_after(
        after,
        NtLoadOrderEntry {
            name,
            image_path,
            group: None,
            tag: None,
            reason: "Kernel binary".to_string(),
            is_kernel_binary: true,
        },
    )
}
