use anyhow::Result;
use std::collections::HashMap;
use std::fs;

use crate::types::Package;
use crate::utils;

pub struct Registry {
    packages: HashMap<String, Package>,
}

impl Registry {
    pub fn load(global: bool) -> Result<Self> {
        let file = utils::get_registry_file(global)?;

        if !file.exists() {
            return Ok(Self {
                packages: HashMap::new(),
            });
        }

        let content = fs::read_to_string(file)?;
        let packages: HashMap<String, Package> = serde_json::from_str(&content)?;

        Ok(Self { packages })
    }

    pub fn save(&self, global: bool) -> Result<()> {
        let file = utils::get_registry_file(global)?;
        let content = serde_json::to_string_pretty(&self.packages)?;
        fs::write(file, content)?;
        Ok(())
    }

    pub fn get(&self, key: &str) -> Option<&Package> {
        self.packages.get(key)
    }

    pub fn insert(&mut self, key: String, package: Package) {
        self.packages.insert(key, package);
    }

    pub fn remove(&mut self, key: &str) -> Option<Package> {
        self.packages.remove(key)
    }

    pub fn is_empty(&self) -> bool {
        self.packages.is_empty()
    }

    pub fn len(&self) -> usize {
        self.packages.len()
    }

    pub fn iter(&self) -> std::collections::hash_map::Iter<String, Package> {
        self.packages.iter()
    }

    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.packages.keys()
    }
}
