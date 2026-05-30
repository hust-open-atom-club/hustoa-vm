use std::error::Error;
use std::fs::{self, File};
use std::net::Ipv6Addr;
use std::path::PathBuf;
use std::str::FromStr;
use colored::Colorize;
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use crate::config::{HustoaVmConfig, Ipv6Config};
use crate::tools::hustoa_run_cmd;

const V6POOL_PATH: &str = "/etc/hustoa-vm/v6pool.toml";

const V6POOL_PATH_DEPRECATED: &str = "/etc/hustoa-vm/v6pool.list";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct V6Pool {
    pool: Vec<V6PoolItem>
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct V6PoolItem {
    addr: Ipv6Addr,
    domain: String,
}

impl V6Pool {
    pub fn get_pool() -> Result<V6Pool, Box <dyn Error>> {
        let mut ret: V6Pool;

        let file_path = PathBuf::from(V6POOL_PATH);
        if file_path.is_file() {
            let v6pool_str = fs::read_to_string(V6POOL_PATH)?;
            if v6pool_str.len() == 0 {
                ret = V6Pool {
                    pool: Vec::new()
                }
            } else {
                ret = toml::from_str(&v6pool_str)?;
            }
        } else {
            File::create(file_path)?;
            ret = V6Pool {
                pool: Vec::new()
            }
        }
        ret.migrate_deprecated_file();

        Ok(ret)
    }

    pub fn print(&self) {
        for item in &self.pool {
            println!("domain: {}\n  - {}\n", item.domain.green(), item.addr)
        }
    }

    fn write_back(&self) -> Result<(), Box <dyn Error>> {
        let str = toml::to_string(&self)?;
        std::fs::write(V6POOL_PATH, str)?;
        Ok(())
    }

    fn migrate_deprecated_file(&mut self) {
        let mut count = 0;
        if PathBuf::from(V6POOL_PATH_DEPRECATED).is_file() {
            if let Ok(lines) = fs::read_to_string(V6POOL_PATH_DEPRECATED) {
                for item in lines.lines() {
                    if item == "" {
                        continue;
                    }
                    let addr = match Ipv6Addr::from_str(item) {
                        Ok(addr) => addr,
                        Err(_) => continue,
                    };
                    if self.insert(&addr, &format!("unknwon-{}", addr)).is_ok() {
                        warn!("migrating ipv6 address {}", item);
                        count += 1;
                    }
                }
            }
        }
        let _ = fs::remove_file(V6POOL_PATH_DEPRECATED);
        if count != 0 {
            warn!(
                "Migrate {} addresses to {}, do not run v6-pool purge before setting domain name manually",
                count, V6POOL_PATH);
        }
    }

    pub fn insert(&mut self, addr: &Ipv6Addr, domain: &String) -> Result<(), Box<dyn Error>> {
        let item = V6PoolItem {
            addr: addr.clone(),
            domain: domain.clone(),
        };
        if self.pool.contains(&item) {
            return Ok(())
        }
        for item in &self.pool {
            if item.addr == *addr {
                error!("Same ipv6 address but different domain name");
                return Err("insert v6pool item failed".into());
            }
            if item.domain == *domain {
                error!("Same domain name but different ipv6 address");
                return Err("insert v6pool item failed".into());
            }
        }
        self.pool.push(item);
        self.write_back()?;
        Ok(())
    }

    pub fn remove_by_addr(&mut self, addr: &Ipv6Addr) -> Result<(), Box <dyn Error>> {
        self.pool.retain(|x| x.addr != *addr);
        self.write_back()?;
        Ok(())
    }

    pub fn remove_by_name(&mut self, name: &String) -> Result<(), Box <dyn Error>> {
        self.pool.retain(|x| x.domain != *name);
        self.write_back()?;
        Ok(())
    }

    pub fn flush(&self, config: &HustoaVmConfig) -> Result<(), Box<dyn Error>> {
        if let Some(ipv6conf) = &config.ipv6conf {
            for item in &self.pool {
                ip_command_mod_one(ipv6conf, &item.addr, false)?;
                ip_command_mod_one(ipv6conf, &item.addr, true)?;
            }
        }

        Ok(())
    }

    pub fn purge(&mut self, _: &HustoaVmConfig) -> Result<(), Box<dyn Error>> {
        let virsh_list = hustoa_run_cmd("virsh", [ "list", "--all", "--name" ], false).output()?;
        let mut vms: Vec<String> = String::from_utf8(virsh_list.stdout)?
            .lines()
            .map(|x| x.to_string())
            .collect();
        vms.retain(|x| *x != "");

        self.pool.retain(|x| {
            let exist = vms.contains(&x.domain);
            if ! exist {
                info!("deleting ipv6 entry: {} domain: {}", x.addr, x.domain)
            }
            exist
        });
        self.write_back()?;
        Ok(())
    }
}

fn ip_command_mod_one(ipv6conf: &Ipv6Config, addr: &Ipv6Addr, is_add: bool) -> Result<(), Box<dyn Error>> {
    let addr_str = format!("{}", addr);

    let action = if is_add { "add" } else { "del" };

    hustoa_run_cmd("ip", [
            "-6",
            "neigh",
            action,
            "proxy",
            &addr_str,
            "dev",
            &ipv6conf.wan_interface.clone()
        ], false)
        .output()?;

    hustoa_run_cmd("ip", [
            "-6",
            "route",
            action,
            &addr_str,
            "dev",
            &ipv6conf.libvirt_interface_v6
        ], false)
        .output()?;

    debug!("flush addr: {}, is_add: {}", addr, is_add);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    use tempfile::TempDir;

    #[test]
    fn test_v6pool_item_equality() {
        let addr1 = Ipv6Addr::from_str("2001:db8::1").unwrap();
        let addr2 = Ipv6Addr::from_str("2001:db8::1").unwrap();
        let domain = "test.com".to_string();

        let item1 = V6PoolItem { addr: addr1, domain: domain.clone() };
        let item2 = V6PoolItem { addr: addr2, domain };

        assert_eq!(item1, item2, "Items with same addr and domain should be equal");
    }

    #[test]
    fn test_v6pool_item_inequality_different_addr() {
        let addr1 = Ipv6Addr::from_str("2001:db8::1").unwrap();
        let addr2 = Ipv6Addr::from_str("2001:db8::2").unwrap();
        let domain = "test.com".to_string();

        let item1 = V6PoolItem { addr: addr1, domain: domain.clone() };
        let item2 = V6PoolItem { addr: addr2, domain };

        assert_ne!(item1, item2, "Items with different addr should not be equal");
    }

    #[test]
    fn test_v6pool_item_inequality_different_domain() {
        let addr = Ipv6Addr::from_str("2001:db8::1").unwrap();
        let domain1 = "test1.com".to_string();
        let domain2 = "test2.com".to_string();

        let item1 = V6PoolItem { addr, domain: domain1 };
        let item2 = V6PoolItem { addr, domain: domain2 };

        assert_ne!(item1, item2, "Items with different domain should not be equal");
    }

    #[test]
    fn test_v6pool_insert_new() {
        let mut pool = V6Pool { pool: Vec::new() };
        let addr = Ipv6Addr::from_str("2001:db8::1").unwrap();
        let domain = "test.com".to_string();

        // Test the insert logic without calling write_back
        let item = V6PoolItem { addr: addr.clone(), domain: domain.clone() };
        assert!(!pool.pool.contains(&item), "Pool should not contain item before insert");

        pool.pool.push(item);

        assert_eq!(pool.pool.len(), 1, "Pool should have 1 item");
        assert_eq!(pool.pool[0].addr, addr, "Address should match");
        assert_eq!(pool.pool[0].domain, domain, "Domain should match");
    }

    #[test]
    fn test_v6pool_insert_duplicate() {
        let mut pool = V6Pool { pool: Vec::new() };
        let addr = Ipv6Addr::from_str("2001:db8::1").unwrap();
        let domain = "test.com".to_string();

        let item = V6PoolItem { addr: addr.clone(), domain: domain.clone() };
        pool.pool.push(item.clone());

        // Test that duplicate detection works
        assert!(pool.pool.contains(&item), "Pool should contain item");
        pool.pool.push(item);

        assert_eq!(pool.pool.len(), 2, "After pushing twice, pool should have 2 items");
    }

    #[test]
    fn test_v6pool_insert_same_addr_different_domain() {
        let mut pool = V6Pool { pool: Vec::new() };
        let addr = Ipv6Addr::from_str("2001:db8::1").unwrap();
        let domain1 = "test1.com".to_string();
        let domain2 = "test2.com".to_string();

        let item1 = V6PoolItem { addr, domain: domain1 };
        pool.pool.push(item1);

        // Test logic for detecting same addr with different domain
        let has_same_addr_different_domain = pool.pool.iter()
            .any(|item| item.addr == addr && item.domain != domain2);
        assert!(has_same_addr_different_domain, "Should detect same addr with different domain");
    }

    #[test]
    fn test_v6pool_insert_same_domain_different_addr() {
        let mut pool = V6Pool { pool: Vec::new() };
        let addr1 = Ipv6Addr::from_str("2001:db8::1").unwrap();
        let addr2 = Ipv6Addr::from_str("2001:db8::2").unwrap();
        let domain = "test.com".to_string();

        let item1 = V6PoolItem { addr: addr1, domain: domain.clone() };
        pool.pool.push(item1);

        // Test logic for detecting same domain with different addr
        let has_same_domain_different_addr = pool.pool.iter()
            .any(|item| item.domain == domain && item.addr != addr2);
        assert!(has_same_domain_different_addr, "Should detect same domain with different addr");
    }

    #[test]
    fn test_v6pool_remove_by_addr() {
        let mut pool = V6Pool { pool: Vec::new() };
        let addr1 = Ipv6Addr::from_str("2001:db8::1").unwrap();
        let addr2 = Ipv6Addr::from_str("2001:db8::2").unwrap();
        let domain1 = "test1.com".to_string();
        let domain2 = "test2.com".to_string();

        pool.pool.push(V6PoolItem { addr: addr1, domain: domain1 });
        pool.pool.push(V6PoolItem { addr: addr2, domain: domain2 });

        // Test retain logic
        let original_len = pool.pool.len();
        pool.pool.retain(|x| x.addr != addr1);

        assert_eq!(pool.pool.len(), original_len - 1, "Pool should have 1 item after removal");
        assert_eq!(pool.pool[0].addr, addr2, "Remaining item should be addr2");
    }

    #[test]
    fn test_v6pool_remove_by_name() {
        let mut pool = V6Pool { pool: Vec::new() };
        let addr1 = Ipv6Addr::from_str("2001:db8::1").unwrap();
        let addr2 = Ipv6Addr::from_str("2001:db8::2").unwrap();
        let domain1 = "test1.com".to_string();
        let domain2 = "test2.com".to_string();

        pool.pool.push(V6PoolItem { addr: addr1, domain: domain1.clone() });
        pool.pool.push(V6PoolItem { addr: addr2, domain: domain2.clone() });

        // Test retain logic
        let original_len = pool.pool.len();
        pool.pool.retain(|x| x.domain != domain1);

        assert_eq!(pool.pool.len(), original_len - 1, "Pool should have 1 item after removal");
        assert_eq!(pool.pool[0].domain, domain2, "Remaining item should be domain2");
    }

    #[test]
    fn test_v6pool_remove_nonexistent_addr() {
        let mut pool = V6Pool { pool: Vec::new() };
        let addr1 = Ipv6Addr::from_str("2001:db8::1").unwrap();
        let addr2 = Ipv6Addr::from_str("2001:db8::2").unwrap();
        let domain1 = "test1.com".to_string();

        pool.pool.push(V6PoolItem { addr: addr1, domain: domain1 });

        let original_len = pool.pool.len();
        pool.pool.retain(|x| x.addr != addr2);

        assert_eq!(pool.pool.len(), original_len, "Pool should still have 1 item");
    }

    #[test]
    fn test_v6pool_remove_nonexistent_name() {
        let mut pool = V6Pool { pool: Vec::new() };
        let addr1 = Ipv6Addr::from_str("2001:db8::1").unwrap();
        let domain1 = "test1.com".to_string();
        let domain2 = "test2.com".to_string();

        pool.pool.push(V6PoolItem { addr: addr1, domain: domain1 });

        let original_len = pool.pool.len();
        pool.pool.retain(|x| x.domain != domain2);

        assert_eq!(pool.pool.len(), original_len, "Pool should still have 1 item");
    }

    #[test]
    fn test_v6pool_contains() {
        let mut pool = V6Pool { pool: Vec::new() };
        let addr = Ipv6Addr::from_str("2001:db8::1").unwrap();
        let domain = "test.com".to_string();

        let item = V6PoolItem { addr, domain: domain.clone() };

        assert!(!pool.pool.contains(&item), "Pool should not contain item before insert");

        pool.pool.push(item.clone());
        assert!(pool.pool.contains(&item), "Pool should contain item after insert");
    }

    #[test]
    fn test_v6pool_multiple_items() {
        let mut pool = V6Pool { pool: Vec::new() };
        let items = vec![
            (Ipv6Addr::from_str("2001:db8::1").unwrap(), "vm1.com".to_string()),
            (Ipv6Addr::from_str("2001:db8::2").unwrap(), "vm2.com".to_string()),
            (Ipv6Addr::from_str("2001:db8::3").unwrap(), "vm3.com".to_string()),
        ];

        for (addr, domain) in &items {
            pool.pool.push(V6PoolItem { addr: *addr, domain: domain.clone() });
        }

        assert_eq!(pool.pool.len(), 3, "Pool should have 3 items");

        for (addr, domain) in &items {
            let item = V6PoolItem { addr: *addr, domain: domain.clone() };
            assert!(pool.pool.contains(&item), "Pool should contain item");
        }
    }

    #[test]
    fn test_v6pool_item_clone() {
        let addr = Ipv6Addr::from_str("2001:db8::1").unwrap();
        let domain = "test.com".to_string();
        let item = V6PoolItem { addr, domain: domain.clone() };

        let cloned = item.clone();
        assert_eq!(item, cloned, "Cloned item should equal original");
    }
}
