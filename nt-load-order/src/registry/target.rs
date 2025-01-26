use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use nt_hive::{Hive, KeyNode, KeyValue, KeyValueData, KeyValues, NtHiveError, SubKeyNodes};

pub struct TargetRegistryWorker {
    system_hive_data: Vec<u8>,
}

impl TargetRegistryWorker {
    pub fn new(system_root: &str) -> Result<Self> {
        let mut system_path = PathBuf::from(system_root);
        system_path.push("system32");
        system_path.push("config");
        system_path.push("SYSTEM");

        let system_hive_data = std::fs::read(&system_path)
            .with_context(|| format!("Could not read file \"{}\"", system_path.display()))?;

        Ok(Self { system_hive_data })
    }

    pub fn hive(&self) -> Result<TargetRegistryHive> {
        let hive = Hive::new(self.system_hive_data.as_ref()).context("Hive::new failed")?;
        Ok(TargetRegistryHive { hive })
    }
}

pub struct TargetRegistryHive<'d> {
    hive: Hive<&'d [u8]>,
}

impl<'d> TargetRegistryHive<'d> {
    pub fn key_node<'h>(&'h self, path: &str) -> Result<TargetRegistryKeyNode<'d, 'h>> {
        let root_key_node = self
            .hive
            .root_key_node()
            .context("Hive::root_key_node failed")?;
        let sub_key_node = root_key_node
            .subpath(path)
            .with_context(|| format!("Did not find \"{path}\" key"))?
            .with_context(|| format!("KeyNode::subpath failed for \"{path}\" key"))?;
        let name = sub_key_node
            .name()
            .with_context(|| format!("Failed to get name of \"{path}\" key"))?
            .to_string_lossy();

        Ok(TargetRegistryKeyNode {
            name,
            key_node: sub_key_node,
        })
    }
}

pub struct TargetRegistryKeyNode<'d, 'h> {
    name: String,
    key_node: KeyNode<'h, &'d [u8]>,
}

impl<'d, 'h> TargetRegistryKeyNode<'d, 'h> {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn subkey(&self, name: &str) -> Result<TargetRegistryKeyNode<'d, 'h>> {
        let sub_key_node = self
            .key_node
            .subkey(name)
            .with_context(|| format!("Key node \"{}\" has no subkey \"{name}\"", self.name))?
            .with_context(|| {
                format!(
                    "KeyNode::subkey failed for subkey \"{name}\" of key node \"{}\"",
                    self.name
                )
            })?;

        Ok(TargetRegistryKeyNode {
            name: name.to_string(),
            key_node: sub_key_node,
        })
    }

    pub fn subkeys(&self) -> Result<TargetRegistrySubKeys<'d, 'h>> {
        let sub_key_nodes = self
            .key_node
            .subkeys()
            .with_context(|| format!("Key node \"{}\" has no subkeys", self.name))?
            .with_context(|| format!("KeyNode::subkeys failed for key node \"{}\"", self.name))?;

        Ok(TargetRegistrySubKeys { sub_key_nodes })
    }

    pub fn value(&self, name: &str) -> Result<TargetRegistryKeyValue<'d, 'h>> {
        let key_value = self
            .key_node
            .value(name)
            .with_context(|| format!("Key node \"{}\" has no value \"{name}\"", self.name))?
            .with_context(|| {
                format!(
                    "KeyNode::value failed for value \"{name}\" of key node \"{}\"",
                    self.name
                )
            })?;

        Ok(TargetRegistryKeyValue {
            name: name.to_string(),
            key_value,
        })
    }

    pub fn values(&self) -> Result<TargetRegistryKeyValues<'d, 'h>> {
        let key_values = self
            .key_node
            .values()
            .with_context(|| format!("Key node \"{}\" has no values", self.name))?
            .with_context(|| format!("KeyNode::values failed for key node \"{}\"", self.name))?;

        Ok(TargetRegistryKeyValues { key_values })
    }
}

pub struct TargetRegistryKeyValue<'d, 'h> {
    name: String,
    key_value: KeyValue<'h, &'d [u8]>,
}

impl TargetRegistryKeyValue<'_, '_> {
    pub fn binary_data(&self) -> Result<Vec<u8>> {
        let key_value_data = self
            .key_value
            .data()
            .with_context(|| format!("KeyValue::data failed for value \"{}\"", self.name))?;

        match key_value_data {
            KeyValueData::Small(small_data) => Ok(small_data.to_vec()),
            KeyValueData::Big(_) => {
                bail!(
                    "KeyValue::data returned big data for value \"{}\", which is not supported",
                    self.name
                )
            }
        }
    }

    pub fn dword_data(&self) -> Result<u32> {
        let data = self
            .key_value
            .dword_data()
            .with_context(|| format!("KeyValue::dword_data failed for value \"{}\"", self.name))?;
        Ok(data)
    }

    pub fn multi_sz_data(&self) -> Result<Vec<String>> {
        let data = self
            .key_value
            .multi_string_data()
            .with_context(|| {
                format!(
                    "KeyValue::multi_string_data failed for value \"{}\"",
                    self.name,
                )
            })?
            .collect::<Result<Vec<String>, NtHiveError>>()?;
        Ok(data)
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn sz_data(&self) -> Result<String> {
        let data = self
            .key_value
            .string_data()
            .with_context(|| format!("KeyValue::string_data failed for value \"{}\"", self.name))?;
        Ok(data)
    }
}

pub struct TargetRegistryKeyValues<'d, 'h> {
    key_values: KeyValues<'h, &'d [u8]>,
}

impl<'d, 'h> Iterator for TargetRegistryKeyValues<'d, 'h> {
    type Item = Result<TargetRegistryKeyValue<'d, 'h>>;

    fn next(&mut self) -> Option<Self::Item> {
        let item = self.key_values.next()?;

        let result = item
            .context("Failed to iterate key value")
            .and_then(|key_value| {
                let name = key_value
                    .name()
                    .context("Failed to get name of iterated key value")?
                    .to_string_lossy();

                Ok(TargetRegistryKeyValue { name, key_value })
            });

        Some(result)
    }
}

pub struct TargetRegistrySubKeys<'d, 'h> {
    sub_key_nodes: SubKeyNodes<'h, &'d [u8]>,
}

impl<'d, 'h> Iterator for TargetRegistrySubKeys<'d, 'h> {
    type Item = Result<TargetRegistryKeyNode<'d, 'h>>;

    fn next(&mut self) -> Option<Self::Item> {
        let item = self.sub_key_nodes.next()?;

        let result = item
            .context("Failed to iterate sub key")
            .and_then(|key_node| {
                let name = key_node
                    .name()
                    .context("Failed to get name of iterated sub key")?
                    .to_string_lossy();
                Ok(TargetRegistryKeyNode { name, key_node })
            });

        Some(result)
    }
}
