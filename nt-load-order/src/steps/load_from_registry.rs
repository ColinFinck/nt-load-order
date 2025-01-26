use std::collections::HashMap;
use std::mem;

use anyhow::Result;
use indexmap::IndexSet;

use crate::registry::{RegistryKeyNode, RegistryKeyValue, RegistryWorker};
use crate::{NtLoadOrderEntry, NtLoadOrderEntryGroup};

pub struct RegistryInfo {
    pub entries: Vec<NtLoadOrderEntry>,
    pub groups: HashMap<String, IndexSet<u32>>,
    pub service_group_order: Vec<String>,
}

pub fn load_from_registry(
    registry_worker: &RegistryWorker,
    boot_file_system: &str,
    control_set: u8,
) -> Result<RegistryInfo> {
    const SERVICE_BOOT_START: u32 = 0;

    let control_set_key_name = format!("ControlSet{control_set:03}");
    let hive = registry_worker.hive()?;

    let hardware_config_id_string = hive
        .key_node("HardwareConfig")?
        .value("LastId")?
        .dword_data()?
        .to_string();

    let service_group_order = hive
        .key_node(&format!(
            "{control_set_key_name}\\Control\\ServiceGroupOrder"
        ))?
        .value("List")?
        .multi_sz_data()?;

    //
    let group_order_list_key_node =
        hive.key_node(&format!("{control_set_key_name}\\Control\\GroupOrderList"))?;
    let group_order_list_values = group_order_list_key_node.values()?;
    let mut groups = HashMap::new();

    for group in group_order_list_values {
        let group = group?;
        let set = get_group_set(&group)?;

        let group_search_key = group.name().to_ascii_lowercase();
        groups.insert(group_search_key, set);
    }

    //
    let services_key_node = hive.key_node(&format!("{control_set_key_name}\\Services"))?;
    let services_key_subkeys = services_key_node.subkeys()?;

    let mut entries = Vec::new();

    for service in services_key_subkeys {
        let service = service?;

        let mut start_and_reason = None;

        // We first need to fetch the data of the "Start" value to later check if this service
        // is a boot driver.
        if let Ok(start_value) = service.value("Start") {
            if let Ok(start_dword) = start_value.dword_data() {
                start_and_reason = Some((start_dword, "Boot Driver via its \"Start\" value"));
            }
        }

        // This value may be overridden on a per-hardware-config basis in the "StartOverride" subkey.
        // Check this key as well.
        if let Ok(start_override) = service.subkey("StartOverride") {
            if let Ok(start_override_value) = start_override.value(&hardware_config_id_string) {
                if let Ok(start_override_dword) = start_override_value.dword_data() {
                    start_and_reason = Some((
                        start_override_dword,
                        "Boot Driver via its value in the \"StartOverride\" subkey",
                    ));
                }
            }
        }

        // Now only add this service to the list if it's really a boot driver.
        if let Some((start, reason)) = start_and_reason {
            if start == SERVICE_BOOT_START {
                add_service(&mut entries, &service, reason.to_string())?;
            }
        }
    }

    // Add the boot file system as well.
    let boot_file_system_node = services_key_node.subkey(boot_file_system)?;
    let reason = "Boot File System Driver";
    add_service(&mut entries, &boot_file_system_node, reason.to_string())?;

    Ok(RegistryInfo {
        entries,
        groups,
        service_group_order,
    })
}

fn get_group_set(group: &RegistryKeyValue) -> Result<IndexSet<u32>> {
    let data = group.binary_data()?;
    let mut set = IndexSet::new();

    if data.len() >= 2 * mem::size_of::<u32>() {
        const U32_SIZE: usize = mem::size_of::<u32>();

        let count = u32::from_le_bytes(data[..U32_SIZE].try_into().unwrap());
        set = data[U32_SIZE..]
            .chunks(U32_SIZE)
            .take(count as usize)
            .map(|x| u32::from_le_bytes(x.try_into().unwrap()))
            .collect::<IndexSet<u32>>();
    }

    Ok(set)
}

fn add_service(
    entries: &mut Vec<NtLoadOrderEntry>,
    service: &RegistryKeyNode,
    reason: String,
) -> Result<()> {
    let name = service.name().to_string();
    let image_path = service_image_path(&service);

    let mut group = None;
    if let Ok(value) = service.value("Group") {
        if let Ok(display_name) = value.sz_data() {
            let search_key = display_name.to_ascii_lowercase();

            group = Some(NtLoadOrderEntryGroup {
                display_name,
                search_key,
            });
        }
    }

    let mut tag = None;
    if let Ok(value) = service.value("Tag") {
        if let Ok(dword) = value.dword_data() {
            tag = Some(dword);
        }
    }

    entries.push(NtLoadOrderEntry {
        name,
        image_path,
        group,
        tag,
        reason,
        is_kernel_binary: false,
    });

    Ok(())
}

fn service_image_path(service: &RegistryKeyNode) -> String {
    // If there is an "ImagePath" value, use that.
    if let Ok(value) = service.value("ImagePath") {
        if let Ok(string) = value.sz_data() {
            return string;
        }
    }

    // Otherwise, derive the image path from the service name.
    // This is actually required for "Fs_Rec" and "Wof".
    format!("System32\\Drivers\\{}.sys", service.name())
}
