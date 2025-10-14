use std::collections::HashSet;
use std::error::Error;
use std::fs::File;
use std::io::{self, BufRead};
use std::net::Ipv6Addr;
use std::path::PathBuf;
use std::process::Command;
use std::str::FromStr;
use log::debug;
use crate::config::{HustoaVmConfig, Ipv6Config};

const V6POOL_PATH: &str = "/etc/hustoa-vm/v6pool.list";

pub struct V6Pool {
    pool: HashSet<Ipv6Addr>
}

impl V6Pool {
    pub fn get_pool() -> Result<V6Pool, Box <dyn Error>> {
        let file_path = PathBuf::from(V6POOL_PATH);
        if !file_path.is_file() {
            File::create(file_path)?;
            return Ok(V6Pool {
                pool: HashSet::new()
            })
        }
        let file = File::open(V6POOL_PATH)?;
        let mut pool = HashSet::new();
        for line in io::BufReader::new(file).lines().flatten() {
            let addr = line.replace("\n", "");
            pool.insert(Ipv6Addr::from_str(&addr)?);
        }
        Ok(V6Pool {
            pool
        })
    }

    fn write_back(&self) -> Result<(), Box <dyn Error>> {
        let mut content = String::new();
        for addr in &self.pool {
            content += addr.to_string().as_str();
            content += "\n";
        }
        std::fs::write(V6POOL_PATH, content)?;
        Ok(())
    }

    pub fn insert(&mut self, addr: Ipv6Addr) -> Result<(), Box <dyn Error>> {
        self.pool.insert(addr);
        self.write_back()?;
        Ok(())
    }

    pub fn remove(&mut self, addr: Ipv6Addr) -> Result<(), Box <dyn Error>> {
        self.pool.remove(&addr);
        self.write_back()?;
        Ok(())
    }

    pub fn flush(&self, config: &HustoaVmConfig) -> Result<(), Box<dyn Error>> {
        if let Some(ipv6conf) = &config.ipv6conf {
            for addr in &self.pool {
                ip_command_mod_one(ipv6conf, addr, false)?;
                ip_command_mod_one(ipv6conf, addr, true)?;
            }
        }

        Ok(())
    }
}

fn ip_command_mod_one(ipv6conf: &Ipv6Config, addr: &Ipv6Addr, is_add: bool) -> Result<(), Box<dyn Error>> {
    let addr_str = format!("{}", addr);

    let action = if is_add { "add" } else { "del" };

    Command::new("ip")
    .args([
        "-6",
        "neigh",
        action,
        "proxy",
        &addr_str,
        "dev",
        &ipv6conf.wan_interface.clone()
    ])
    .output()?;

    Command::new("ip")
    .args([
        "-6",
        "route",
        action,
        &addr_str,
        "dev",
        &ipv6conf.libvirt_interface_v6
    ])
    .output()?;

    debug!("flush addr: {}, is_add: {}", addr, is_add);

    Ok(())
}
