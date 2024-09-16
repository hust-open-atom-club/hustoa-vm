use std::error::Error;
use std::fs::{self, File};
use std::net::Ipv6Addr;
use std::path::PathBuf;
use std::str::FromStr;
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
        let file_path = PathBuf::from(V6POOL_PATH);
        if !file_path.is_file() {
            File::create(file_path)?;
            return Ok(V6Pool {
                pool: Vec::new()
            })
        }
        let v6pool_str = fs::read_to_string(V6POOL_PATH)?;
        if v6pool_str.len() == 0 {
            return Ok(V6Pool {
                pool: Vec::new()
            })
        }
        let mut ret: V6Pool = toml::from_str(&v6pool_str)?;

        ret.migrate_deprecated_file();

        Ok(ret)
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
