mod local;
mod target;

use anyhow::Result;

use self::local::{
    LocalRegistryHive, LocalRegistryKeyNode, LocalRegistryKeyValue, LocalRegistryKeyValues,
    LocalRegistrySubKeys, LocalRegistryWorker,
};
use self::target::{
    TargetRegistryHive, TargetRegistryKeyNode, TargetRegistryKeyValue, TargetRegistryKeyValues,
    TargetRegistrySubKeys, TargetRegistryWorker,
};

pub enum RegistryWorker {
    #[cfg(target_os = "windows")]
    Local(LocalRegistryWorker),
    Target(TargetRegistryWorker),
}

impl RegistryWorker {
    #[cfg(target_os = "windows")]
    pub fn new_local() -> Self {
        let worker = LocalRegistryWorker;
        Self::Local(worker)
    }

    pub fn new_target(system_root: &str) -> Result<Self> {
        let worker = TargetRegistryWorker::new(system_root)?;
        Ok(Self::Target(worker))
    }

    pub fn hive(&self) -> Result<RegistryHive> {
        match self {
            #[cfg(target_os = "windows")]
            Self::Local(worker) => worker.hive().map(|local| RegistryHive::Local(local)),
            Self::Target(worker) => worker.hive().map(|target| RegistryHive::Target(target)),
        }
    }
}

pub enum RegistryHive<'d> {
    #[cfg(target_os = "windows")]
    Local(LocalRegistryHive),
    Target(TargetRegistryHive<'d>),
}

impl<'d> RegistryHive<'d> {
    pub fn key_node<'h>(&'h self, path: &str) -> Result<RegistryKeyNode<'d, 'h>> {
        match self {
            #[cfg(target_os = "windows")]
            Self::Local(hive) => hive
                .key_node(path)
                .map(|local| RegistryKeyNode::Local(local)),
            Self::Target(hive) => hive
                .key_node(path)
                .map(|target| RegistryKeyNode::Target(target)),
        }
    }
}

pub enum RegistryKeyNode<'d, 'h> {
    #[cfg(target_os = "windows")]
    Local(LocalRegistryKeyNode),
    Target(TargetRegistryKeyNode<'d, 'h>),
}

impl<'d, 'h> RegistryKeyNode<'d, 'h> {
    pub fn name(&self) -> &str {
        match self {
            #[cfg(target_os = "windows")]
            Self::Local(key_node) => key_node.name(),
            Self::Target(key_node) => key_node.name(),
        }
    }

    pub fn subkey(&self, name: &str) -> Result<RegistryKeyNode<'d, 'h>> {
        match self {
            #[cfg(target_os = "windows")]
            Self::Local(key_node) => key_node
                .subkey(name)
                .map(|local| RegistryKeyNode::Local(local)),
            Self::Target(key_node) => key_node
                .subkey(name)
                .map(|target| RegistryKeyNode::Target(target)),
        }
    }

    pub fn subkeys<'n>(&'n self) -> Result<RegistrySubKeys<'d, 'h, 'n>> {
        match self {
            #[cfg(target_os = "windows")]
            Self::Local(key_node) => {
                Ok(key_node.subkeys()).map(|local| RegistrySubKeys::Local(local))
            }
            Self::Target(key_node) => key_node
                .subkeys()
                .map(|target| RegistrySubKeys::Target(target)),
        }
    }

    pub fn value(&self, name: &str) -> Result<RegistryKeyValue<'d, 'h>> {
        match self {
            #[cfg(target_os = "windows")]
            Self::Local(key_node) => key_node
                .value(name)
                .map(|local| RegistryKeyValue::Local(local)),
            Self::Target(key_node) => key_node
                .value(name)
                .map(|target| RegistryKeyValue::Target(target)),
        }
    }

    pub fn values<'n>(&'n self) -> Result<RegistryKeyValues<'d, 'h, 'n>> {
        match self {
            #[cfg(target_os = "windows")]
            Self::Local(key_node) => {
                Ok(key_node.values()).map(|local| RegistryKeyValues::Local(local))
            }
            Self::Target(key_node) => key_node
                .values()
                .map(|target| RegistryKeyValues::Target(target)),
        }
    }
}

pub enum RegistryKeyValue<'d, 'h> {
    #[cfg(target_os = "windows")]
    Local(LocalRegistryKeyValue),
    Target(TargetRegistryKeyValue<'d, 'h>),
}

impl<'d, 'h> RegistryKeyValue<'d, 'h> {
    pub fn binary_data(&self) -> Result<Vec<u8>> {
        match self {
            #[cfg(target_os = "windows")]
            Self::Local(value) => value.binary_data(),
            Self::Target(value) => value.binary_data(),
        }
    }

    pub fn dword_data(&self) -> Result<u32> {
        match self {
            #[cfg(target_os = "windows")]
            Self::Local(value) => value.dword_data(),
            Self::Target(value) => value.dword_data(),
        }
    }

    pub fn multi_sz_data(&self) -> Result<Vec<String>> {
        match self {
            #[cfg(target_os = "windows")]
            Self::Local(value) => value.multi_sz_data(),
            Self::Target(value) => value.multi_sz_data(),
        }
    }

    pub fn name(&self) -> &str {
        match self {
            #[cfg(target_os = "windows")]
            Self::Local(value) => value.name(),
            Self::Target(value) => value.name(),
        }
    }

    pub fn sz_data(&self) -> Result<String> {
        match self {
            #[cfg(target_os = "windows")]
            Self::Local(value) => value.sz_data(),
            Self::Target(value) => value.sz_data(),
        }
    }
}

pub enum RegistryKeyValues<'d, 'h, 'n> {
    #[cfg(target_os = "windows")]
    Local(LocalRegistryKeyValues<'n>),
    Target(TargetRegistryKeyValues<'d, 'h>),
}

impl<'d, 'h, 'n> Iterator for RegistryKeyValues<'d, 'h, 'n> {
    type Item = Result<RegistryKeyValue<'d, 'h>>;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            #[cfg(target_os = "windows")]
            Self::Local(iter) => Some(iter.next()?.map(|local| RegistryKeyValue::Local(local))),
            Self::Target(iter) => Some(iter.next()?.map(|target| RegistryKeyValue::Target(target))),
        }
    }
}

pub enum RegistrySubKeys<'d, 'h, 'n> {
    #[cfg(target_os = "windows")]
    Local(LocalRegistrySubKeys<'n>),
    Target(TargetRegistrySubKeys<'d, 'h>),
}

impl<'d, 'h, 'n> Iterator for RegistrySubKeys<'d, 'h, 'n> {
    type Item = Result<RegistryKeyNode<'d, 'h>>;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            #[cfg(target_os = "windows")]
            Self::Local(iter) => Some(iter.next()?.map(|local| RegistryKeyNode::Local(local))),
            Self::Target(iter) => Some(iter.next()?.map(|target| RegistryKeyNode::Target(target))),
        }
    }
}
