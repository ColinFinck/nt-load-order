mod registry;
mod steps;

use anyhow::{Context, Result};
use dlv_list::VecList;

use crate::registry::RegistryWorker;
use crate::steps::{
    add_basic_kernel_binaries, add_imports, add_kernel_binary, load_from_registry,
    sort_by_hardcoded_groups, sort_by_hardcoded_service_lists, sort_by_tag_and_group,
};

#[derive(Clone)]
pub struct NtLoadOrder {
    /// Optional path to a target SystemRoot directory.
    /// If not set, the running operating system is analyzed.
    ///
    /// Defaults to `None`.
    system_root: Option<String>,
    /// Optional KD driver to load (e.g. "kdcom").
    kd_driver: Option<String>,
    /// Optional vendor string of the CPU to run the target operating system (e.g. "AuthenticAMD").
    /// If set, a matching "mcupdate_*.dll" binary will be added to the loaded kernel binaries.
    cpu_vendor: Option<String>,
    /// Whether to sort the fetched services by their tags
    /// and groups based on the ServiceGroupOrder and
    /// GroupOrderList.
    ///
    /// Defaults to `true`.
    sort_by_tag_and_group: bool,
    /// Whether to sort the fetched services based on
    /// groups hardcoded into the bootloader
    /// (which precede all other groups).
    ///
    /// Defaults to `true`.
    sort_by_hardcoded_groups: bool,
    /// Whether to sort the fetched services based on
    /// service lists hardcoded into the bootloader
    /// (which precede all other groups).
    ///
    /// Defaults to `true`.
    sort_by_hardcoded_service_lists: bool,
    /// Whether to add the kernel (ntoskrnl.exe) and related
    /// hardcoded binaries (hal.dll, kdcom.dll, mcupdate.dll)
    /// in the load order.
    ///
    /// Defaults to `true`.
    add_kernel_binaries: bool,
    /// Whether to add imports of modules in the load order.
    ///
    /// Defaults to `true`.
    add_imports: bool,
}

#[derive(Clone)]
pub struct NtLoadOrderEntry {
    pub name: String,
    pub image_path: String,
    pub group: Option<NtLoadOrderEntryGroup>,
    pub tag: Option<u32>,
    pub reason: String,
    /// The first few kernel binaries have fixed positions that don't move.
    /// Mark them differently here.
    pub is_kernel_binary: bool,
}

#[derive(Clone)]
pub struct NtLoadOrderEntryGroup {
    /// The original name of this group, used for displaying.
    pub display_name: String,
    /// The key used for looking up this group in the `groups` HashMap.
    /// This is a lowercased version of `display_name`.
    pub search_key: String,
}

impl NtLoadOrder {
    pub fn new() -> Self {
        Self {
            system_root: None,
            kd_driver: None,
            cpu_vendor: None,
            sort_by_tag_and_group: true,
            sort_by_hardcoded_groups: true,
            sort_by_hardcoded_service_lists: true,
            add_kernel_binaries: true,
            add_imports: true,
        }
    }

    pub fn add_imports(mut self, value: bool) -> Self {
        self.add_imports = value;
        self
    }

    pub fn add_kernel_binaries(mut self, value: bool) -> Self {
        self.add_kernel_binaries = value;
        self
    }

    pub fn cpu_vendor(mut self, cpu_vendor: Option<String>) -> Self {
        self.cpu_vendor = cpu_vendor;
        self
    }

    pub fn kd_driver(mut self, kd_driver: Option<String>) -> Self {
        self.kd_driver = kd_driver;
        self
    }

    pub fn sort_by_hardcoded_groups(mut self, value: bool) -> Self {
        self.sort_by_hardcoded_groups = value;
        self
    }

    pub fn sort_by_hardcoded_service_lists(mut self, value: bool) -> Self {
        self.sort_by_hardcoded_service_lists = value;
        self
    }

    pub fn sort_by_tag_and_group(mut self, value: bool) -> Self {
        self.sort_by_tag_and_group = value;
        self
    }

    pub fn system_root(mut self, system_root: Option<String>) -> Self {
        self.system_root = system_root;
        self
    }

    pub fn get(self) -> Result<Vec<NtLoadOrderEntry>> {
        // Hardcoded for now, but will work for 99.9% of the cases :)
        const BOOT_FILE_SYSTEM: &str = "ntfs";
        const CONTROL_SET: u8 = 1;

        let registry_worker = if let Some(system_root) = &self.system_root {
            // Load services from target registry.
            RegistryWorker::new_target(system_root)?
        } else {
            // Load services from local registry.
            RegistryWorker::new_local()
        };

        let registry_info = load_from_registry(&registry_worker, BOOT_FILE_SYSTEM, CONTROL_SET)?;

        let mut entries = if self.sort_by_tag_and_group {
            sort_by_tag_and_group(registry_info)
        } else {
            registry_info.entries.into_iter().collect::<VecList<_>>()
        };

        if self.sort_by_hardcoded_groups {
            sort_by_hardcoded_groups(&mut entries);
        }

        if self.sort_by_hardcoded_service_lists {
            sort_by_hardcoded_service_lists(&mut entries);
        }

        if self.add_kernel_binaries {
            let mut last = add_basic_kernel_binaries(&mut entries);

            if let Some(kd_driver) = &self.kd_driver {
                last = add_kernel_binary(
                    &mut entries,
                    last,
                    kd_driver.clone(),
                    format!("System32\\{kd_driver}.dll"),
                );
            }

            if let Some(cpu_vendor) = &self.cpu_vendor {
                add_kernel_binary(
                    &mut entries,
                    last,
                    "mcupdate".to_string(),
                    format!("System32\\mcupdate_{cpu_vendor}.dll"),
                );
            }
        }

        if self.add_imports {
            let system_root = if let Some(system_root) = &self.system_root {
                // Load imports from the target system root.
                system_root.clone()
            } else {
                // Get the local system root from the environment variable.
                std::env::var("SystemRoot")
                    .context("Could not read SystemRoot environment variable")?
            };

            entries = add_imports(entries, system_root)?;
        }

        Ok(entries.into_iter().collect())
    }
}

impl Default for NtLoadOrder {
    fn default() -> Self {
        Self::new()
    }
}
