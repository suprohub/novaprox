use std::{fs, net::IpAddr, path::Path};

use ahash::{HashMap, HashMapExt as _};
use anyhow::{Context as _, Result};

pub struct DnsCache {
    cache: HashMap<String, (IpAddr, bool)>,
    cache_file: String,
}

impl Default for DnsCache {
    fn default() -> Self {
        Self::new("resolved.txt")
    }
}

impl DnsCache {
    #[must_use]
    pub fn new(cache_file: &str) -> Self {
        Self {
            cache: HashMap::new(),
            cache_file: cache_file.to_owned(),
        }
    }

    pub fn load_cache(&mut self) -> Result<HashMap<String, (IpAddr, bool)>> {
        if !Path::new(&self.cache_file).exists() {
            return Ok(HashMap::new());
        }

        fs::read_to_string(&self.cache_file)
            .context("Failed to read DNS cache")?
            .lines()
            .filter_map(|line| {
                let mut parts = line.split_whitespace();
                let domain = parts.next()?;
                let ip = parts.next()?.parse().ok()?;
                Some((domain.to_owned(), (ip, false)))
            })
            .try_fold(HashMap::new(), |mut map, (domain, entry)| {
                map.insert(domain, entry);
                Ok(map)
            })
    }

    pub fn get(&mut self, domain: &str) -> Option<IpAddr> {
        self.cache.get_mut(domain).map(|(ip, used)| {
            *used = true;
            *ip
        })
    }

    pub fn insert(&mut self, domain: String, ip: IpAddr) -> Option<IpAddr> {
        self.cache.insert(domain, (ip, true)).map(|(ip, _)| ip)
    }

    pub fn save(&self) -> Result<()> {
        let content = self
            .cache
            .iter()
            .filter(|(_, (_, used))| *used)
            .map(|(domain, (ip, _))| format!("{domain} {ip}"))
            .collect::<Vec<_>>()
            .join("\n");

        fs::write(&self.cache_file, content).context("Failed to save DNS cache")
    }
}
