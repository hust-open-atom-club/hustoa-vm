use serde::Deserialize;
use std::path::PathBuf;
use std::{fs, net::Ipv6Addr};
use std::error::Error;
use log::debug;

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

    #[serde(default = "default_libvirt_network")]
    pub libvirt_interface: String,
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

fn default_libvirt_network() -> String {
    String::from("virbr0")
}

impl HustoaVmConfig {
    pub fn get_global_config() -> Result<HustoaVmConfig, Box<dyn Error>> {
        let toml_str = fs::read_to_string(DEFAULT_CONFIG_PATH)?;
        let config: HustoaVmConfig = toml::from_str(&toml_str)?;
        debug!("{:?}", config);
        Ok(config)
    }
}
