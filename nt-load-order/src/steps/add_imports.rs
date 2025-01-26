// Copyright 2025 Colin Finck <colin@reactos.org>
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::collections::HashSet;
use std::path::Path;

use anyhow::{bail, Context, Result};
use dlv_list::VecList;
use nt_apiset::ApiSetMap;
use pelite::pe64::{Pe, PeFile};
use pelite::FileMap;

use crate::NtLoadOrderEntry;

pub fn add_imports(
    mut entries: VecList<NtLoadOrderEntry>,
    system_root: String,
) -> Result<VecList<NtLoadOrderEntry>> {
    // Prepare the path handler.
    let path_handler = PathHandler::new(system_root);

    // Load the apisetschema.dll
    let apisetschema_file_path = path_handler.full_path_name("System32\\apisetschema.dll");
    let apisetschema_file_map = FileMap::open(&apisetschema_file_path)
        .with_context(|| format!("FileMap::open failed for \"{apisetschema_file_path}\""))?;
    let apisetschema_pe_file = PeFile::from_bytes(&apisetschema_file_map)
        .with_context(|| format!("PeFile::from_bytes failed for \"{apisetschema_file_path}\""))?;
    let apiset_map = ApiSetMap::try_from_pe64(apisetschema_pe_file).with_context(|| {
        format!("ApiSetMap::try_from_pe64 failed for \"{apisetschema_file_path}\"")
    })?;

    // Prepare the import handler.
    let mut import_handler = ImportHandler::new(&path_handler, apiset_map);

    // The hardcoded kernel binaries are treated differently than the remaining services.
    // They have fixed positions at the beginning of the list and don't move anymore.
    // Achieve this by adding them to `loaded_image_paths` before calling `handle_image`.
    for entry in entries.iter().take_while(|entry| entry.is_kernel_binary) {
        import_handler
            .loaded_image_paths
            .insert(entry.image_path.to_ascii_lowercase());
        import_handler.entries.push_back(entry.clone());
    }

    // Now add the imports of the passed kernel binaries.
    let mut drain = entries.drain();
    let mut current = drain.next();

    while let Some(entry) = &current {
        if !entry.is_kernel_binary {
            break;
        }

        import_handler.handle_image(&entry.image_path)?;
        current = drain.next();
    }

    // Handle the remaining services.
    while let Some(entry) = current {
        if import_handler
            .loaded_image_paths
            .insert(entry.image_path.to_ascii_lowercase())
        {
            // Add the service first, then handle it for adding its imports.
            let entry_image_path = entry.image_path.clone();
            import_handler.entries.push_back(entry);
            import_handler.handle_image(&entry_image_path)?;
        }

        current = drain.next();
    }

    Ok(import_handler.entries)
}

struct PathHandler {
    system_root: String,
}

impl PathHandler {
    fn new(system_root: String) -> Self {
        Self { system_root }
    }

    fn full_path_name(&self, image_path: &str) -> String {
        format!("{}\\{image_path}", self.system_root)
    }

    fn get_image_path(&self, file_name: &str) -> Result<String> {
        // Look in "system32\drivers"
        let image_path = format!("System32\\drivers\\{file_name}");
        let check_path = self.full_path_name(&image_path);
        if Path::new(&check_path).exists() {
            return Ok(image_path);
        }

        // Look in "system32"
        let image_path = format!("System32\\{file_name}");
        let check_path = self.full_path_name(&image_path);
        if Path::new(&check_path).exists() {
            return Ok(image_path);
        }

        // Give up.
        bail!("Cannot find \"{file_name}\" in {}", self.system_root)
    }
}

struct ImportHandler<'a, 'b> {
    apiset_map: ApiSetMap<'b>,
    entries: VecList<NtLoadOrderEntry>,
    loaded_image_paths: HashSet<String>,
    path_handler: &'a PathHandler,
}

impl<'a, 'b> ImportHandler<'a, 'b> {
    fn new(path_handler: &'a PathHandler, apiset_map: ApiSetMap<'b>) -> Self {
        Self {
            apiset_map,
            entries: VecList::new(),
            loaded_image_paths: HashSet::new(),
            path_handler,
        }
    }

    fn handle_image(&mut self, image_path: &str) -> Result<()> {
        // Open the file as a PE file.
        let file_path = self.path_handler.full_path_name(image_path);
        let file_map = FileMap::open(&file_path)
            .with_context(|| format!("FileMap::open failed for \"{file_path}\""))?;
        let pe_file = PeFile::from_bytes(&file_map)
            .with_context(|| format!("PeFile::from_bytes failed for \"{file_path}\""))?;

        let Ok(imports) = pe_file.imports() else {
            return Ok(());
        };

        for import in imports {
            let dll_name = import
                .dll_name()
                .with_context(|| {
                    "pelite::pe64::imports::Desc::dll_name failed for an import of \"{file_path}\""
                })?
                .to_string();

            let dll_name = self
                .patch_dll_name(dll_name)
                .with_context(|| format!("While handling imports of \"{file_path}\""))?;

            let Some(dll_name) = dll_name else {
                // An API Set Map lookup revealed that this import is not available on this operating system.
                // It is therefore ignored by the PE loader.
                continue;
            };

            // Determine the image path to the import file name.
            let import_image_path = self.path_handler.get_image_path(&dll_name)?;

            // If this import has not been handled before, handle it now.
            if self
                .loaded_image_paths
                .insert(import_image_path.to_ascii_lowercase())
            {
                // Handle imports of this import first, then add this import.
                //
                // This is exactly opposite to the way it's done for services, and adds to the confusing resulting
                // load order of the Windows bootloader.
                self.handle_image(&import_image_path)?;
                self.entries.push_back(NtLoadOrderEntry {
                    name: dll_name,
                    image_path: import_image_path,
                    group: None,
                    tag: None,
                    reason: format!("Import of \"{image_path}\""),
                    is_kernel_binary: false,
                });
            }
        }

        Ok(())
    }

    /// Looks up the passed import file name in the operating system's API Set Map.
    ///
    /// If the file name does not fulfill the requirements for API Set Map entries, the passed file name is
    /// returned unmodified.
    /// If the file name has an API Set Map entry, the file name of the corresponding entry is returned.
    /// Otherwise, if the file name has no such entry, `None` is returned.
    fn patch_dll_name(&self, dll_name: String) -> Result<Option<String>> {
        let Some(lookup_name) = dll_name.strip_suffix(".dll") else {
            // `dll_name` has no lowercase .dll extension, which is a requirement for having an API Set entry.
            // So return the unmodified `dll_name`.
            return Ok(Some(dll_name));
        };

        if !lookup_name.starts_with("api-") && !lookup_name.starts_with("ext-") {
            // `dll_name` does not start with "api-" or "ext-", which is a requirement for having an API Set entry.
            // So return the unmodified `dll_name`.
            return Ok(Some(dll_name));
        }

        let Some(Ok(namespace_entry)) = self.apiset_map.find_namespace_entry(lookup_name) else {
            // Although `dll_name` has been validated as an API Set, it has no entry in the API Set Map.
            // This indicates that the requested API Set import is not available on this operating system,
            // and should be ignored.
            // So return `None`.
            return Ok(None);
        };

        let mut value_entries = namespace_entry.value_entries().with_context(|| {
            format!("ApiSetNamespaceEntry::value_entries failed for \"{dll_name}\"")
        })?;

        let Some(value_entry) = value_entries.next() else {
            // Although `dll_name` has been validated as an API Set and has a namespace entry, it does not have
            // a single value entry in the API Set Map.
            // Return `None` like above.
            return Ok(None);
        };

        let value = value_entry
            .value()
            .with_context(|| format!("ApiSetValueEntry::value failed for \"{dll_name}\""))?;

        if value.is_empty() {
            // Although `dll_name` has been validated as an API Set and has a namespace entry with a value entry,
            // that value entry is empty.
            // Return `None` like above.
            return Ok(None);
        }

        Ok(Some(value.to_string_lossy()))
    }
}
