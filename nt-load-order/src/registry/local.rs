use anyhow::{Context, Result};
use winreg::enums::HKEY_LOCAL_MACHINE;
use winreg::types::FromRegValue;
use winreg::{EnumKeys, EnumValues, RegKey, RegValue};

pub struct LocalRegistryWorker;

impl LocalRegistryWorker {
    pub fn hive(&self) -> Result<LocalRegistryHive> {
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        let system_key = hklm.open_subkey("SYSTEM")?;
        Ok(LocalRegistryHive { system_key })
    }
}

pub struct LocalRegistryHive {
    system_key: RegKey,
}

impl LocalRegistryHive {
    pub fn key_node<'d, 'h>(&self, path: &str) -> Result<LocalRegistryKeyNode> {
        let key = self.system_key.open_subkey(path)?;
        let name = path.rsplit_once('\\').map(|(_, name)| name).unwrap_or(path);

        Ok(LocalRegistryKeyNode {
            name: name.to_string(),
            key,
        })
    }
}

pub struct LocalRegistryKeyNode {
    name: String,
    key: RegKey,
}

impl LocalRegistryKeyNode {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn subkey<'d, 'h>(&self, name: &str) -> Result<LocalRegistryKeyNode> {
        let key = self.key.open_subkey(name)?;

        Ok(LocalRegistryKeyNode {
            name: name.to_string(),
            key,
        })
    }

    pub fn subkeys<'n>(&'n self) -> LocalRegistrySubKeys<'n> {
        let enum_keys = self.key.enum_keys();

        LocalRegistrySubKeys {
            key: &self.key,
            enum_keys,
        }
    }

    pub fn value(&self, name: &str) -> Result<LocalRegistryKeyValue> {
        let value = self.key.get_raw_value(name)?;

        Ok(LocalRegistryKeyValue {
            name: name.to_string(),
            value,
        })
    }

    pub fn values<'n>(&'n self) -> LocalRegistryKeyValues<'n> {
        let enum_values = self.key.enum_values();
        LocalRegistryKeyValues { enum_values }
    }
}

pub struct LocalRegistryKeyValue {
    name: String,
    value: RegValue,
}

impl LocalRegistryKeyValue {
    pub fn binary_data(&self) -> Result<Vec<u8>> {
        Ok(self.value.bytes.clone())
    }

    pub fn dword_data(&self) -> Result<u32> {
        let data = u32::from_reg_value(&self.value)?;
        Ok(data)
    }

    pub fn multi_sz_data(&self) -> Result<Vec<String>> {
        let data = Vec::<String>::from_reg_value(&self.value)?;
        Ok(data)
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn sz_data(&self) -> Result<String> {
        let data = String::from_reg_value(&self.value)?;
        Ok(data)
    }
}

pub struct LocalRegistryKeyValues<'n> {
    enum_values: EnumValues<'n>,
}

impl<'n> Iterator for LocalRegistryKeyValues<'n> {
    type Item = Result<LocalRegistryKeyValue>;

    fn next(&mut self) -> Option<Self::Item> {
        let item = self.enum_values.next()?;

        let result = item
            .context("Failed to iterate key value")
            .map(|(name, value)| LocalRegistryKeyValue { name, value });

        Some(result)
    }
}

pub struct LocalRegistrySubKeys<'n> {
    key: &'n RegKey,
    enum_keys: EnumKeys<'n>,
}

impl<'n> Iterator for LocalRegistrySubKeys<'n> {
    type Item = Result<LocalRegistryKeyNode>;

    fn next(&mut self) -> Option<Self::Item> {
        let item = self.enum_keys.next()?;

        let result = item.context("Failed to iterate sub key").and_then(|name| {
            let key = self.key.open_subkey(&name)?;
            Ok(LocalRegistryKeyNode { name, key })
        });

        Some(result)
    }
}
