use serde::Deserialize;
use std::path::PathBuf;
use std::{fs, net::Ipv6Addr};
use std::error::Error;
use log::{debug, error};
use lazy_static::lazy_static;

const DEFAULT_CONFIG_PATH: &str = "/etc/hustoa-vm/config.toml";


#[derive(Debug, Deserialize)]
pub struct HustoaVmConfig {
    pub common: CommonConfig,
    pub ipv6conf: Option<Ipv6Config>
}

#[derive(Debug, Deserialize)]
pub struct CommonConfig {
    #[serde(default = "default_libvirt_storage")]
    pub libvirt_storage: PathBuf,

    #[serde(default = "default_libvirt_save")]
    pub libvirt_save: PathBuf,

    #[serde(default = "default_libvirt_network")]
    pub libvirt_network: String,

    #[serde(default = "default_disk_size")]
    pub default_disk_size: usize,

    #[serde(default = "default_vcpus")]
    pub default_vcpus: usize,

    #[serde(default = "default_memory_size")]
    pub default_memory_size: usize,
}

#[derive(Debug, Deserialize)]
pub struct Ipv6Config {
    pub libvirt_interface_v6: String,

    pub ipv6_bridge_mac: String,

    pub ipv6_prefix: Ipv6Addr,

    pub wan_interface: String,
}


fn default_libvirt_storage() -> PathBuf {
    PathBuf::from("/var/lib/libvirt/images")
}

fn default_libvirt_save() -> PathBuf {
    PathBuf::from("/var/lib/libvirt/qemu/save")
}

fn default_libvirt_network() -> String {
    String::from("default")
}

pub fn default_disk_size() -> usize {
    80
}

pub fn default_vcpus() -> usize {
    16
}

pub fn default_memory_size() ->usize {
    16
}

lazy_static! {
    pub static ref global_config: Result<HustoaVmConfig, Box<dyn Error + Send + Sync>> = get_global_config();
}

pub fn get_global_config() -> Result<HustoaVmConfig, Box<dyn Error + Send + Sync>> {
    let toml_str = match fs::read_to_string(DEFAULT_CONFIG_PATH) {
        Ok(strres) => strres,
        Err(msg) => {
            error!("Read config file failed");
            return Err(Box::new(msg))
        }
    };
    let config: HustoaVmConfig = toml::from_str(&toml_str)?;
    debug!("{:?}", config);
    Ok(config)
}
