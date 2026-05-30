use serde::Deserialize;
use std::path::PathBuf;
use std::{fs, net::Ipv6Addr, str::FromStr};
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_default_libvirt_storage() {
        let path = default_libvirt_storage();
        assert_eq!(path, PathBuf::from("/var/lib/libvirt/images"));
    }

    #[test]
    fn test_default_libvirt_save() {
        let path = default_libvirt_save();
        assert_eq!(path, PathBuf::from("/var/lib/libvirt/qemu/save"));
    }

    #[test]
    fn test_default_libvirt_network() {
        let network = default_libvirt_network();
        assert_eq!(network, "default");
    }

    #[test]
    fn test_default_disk_size() {
        let size = default_disk_size();
        assert_eq!(size, 80);
    }

    #[test]
    fn test_default_vcpus() {
        let vcpus = default_vcpus();
        assert_eq!(vcpus, 16);
    }

    #[test]
    fn test_default_memory_size() {
        let memory = default_memory_size();
        assert_eq!(memory, 16);
    }

    #[test]
    fn test_parse_valid_config() {
        let config_str = r#"
[common]
libvirt_storage = "/var/lib/libvirt/images"
libvirt_save = "/var/lib/libvirt/qemu/save"
libvirt_network = "default"
default_disk_size = 100
default_vcpus = 8
default_memory_size = 16

[ipv6conf]
libvirt_interface_v6 = "virbr0-v6"
ipv6_bridge_mac = "52:54:00:12:34:56"
ipv6_prefix = "2001:db8::"
wan_interface = "eth0"
"#;

        let config: HustoaVmConfig = toml::from_str(config_str).unwrap();
        assert_eq!(config.common.libvirt_storage, PathBuf::from("/var/lib/libvirt/images"));
        assert_eq!(config.common.libvirt_save, PathBuf::from("/var/lib/libvirt/qemu/save"));
        assert_eq!(config.common.libvirt_network, "default");
        assert_eq!(config.common.default_disk_size, 100);
        assert_eq!(config.common.default_vcpus, 8);
        assert_eq!(config.common.default_memory_size, 16);
        assert!(config.ipv6conf.is_some());
        let ipv6conf = config.ipv6conf.unwrap();
        assert_eq!(ipv6conf.libvirt_interface_v6, "virbr0-v6");
        assert_eq!(ipv6conf.ipv6_bridge_mac, "52:54:00:12:34:56");
    }

    #[test]
    fn test_parse_config_with_defaults() {
        let config_str = r#"
[common]
libvirt_storage = "/custom/storage"
libvirt_save = "/custom/save"
libvirt_network = "mynetwork"
"#;

        let config: HustoaVmConfig = toml::from_str(config_str).unwrap();
        assert_eq!(config.common.libvirt_storage, PathBuf::from("/custom/storage"));
        assert_eq!(config.common.libvirt_save, PathBuf::from("/custom/save"));
        assert_eq!(config.common.libvirt_network, "mynetwork");
        assert_eq!(config.common.default_disk_size, 80);
        assert_eq!(config.common.default_vcpus, 16);
        assert_eq!(config.common.default_memory_size, 16);
        assert!(config.ipv6conf.is_none());
    }

    #[test]
    fn test_parse_config_minimal() {
        let config_str = r#"
[common]
"#;

        let config: HustoaVmConfig = toml::from_str(config_str).unwrap();
        assert_eq!(config.common.libvirt_storage, PathBuf::from("/var/lib/libvirt/images"));
        assert_eq!(config.common.libvirt_save, PathBuf::from("/var/lib/libvirt/qemu/save"));
        assert_eq!(config.common.libvirt_network, "default");
    }

    #[test]
    fn test_parse_invalid_ipv6_prefix() {
        let config_str = r#"
[common]
libvirt_storage = "/var/lib/libvirt/images"
libvirt_save = "/var/lib/libvirt/qemu/save"

[ipv6conf]
libvirt_interface_v6 = "virbr0-v6"
ipv6_bridge_mac = "52:54:00:12:34:56"
ipv6_prefix = "invalid-ipv6"
wan_interface = "eth0"
"#;

        let result = toml::from_str::<HustoaVmConfig>(config_str);
        assert!(result.is_err(), "Invalid IPv6 prefix should fail to parse");
    }

    #[test]
    fn test_parse_config_invalid_toml() {
        let config_str = r#"
[common
libvirt_storage = "/var/lib/libvirt/images"
"#;

        let result = toml::from_str::<HustoaVmConfig>(config_str);
        assert!(result.is_err(), "Invalid TOML should fail to parse");
    }

    #[test]
    fn test_ipv6_config_fields() {
        let config_str = r#"
[common]
libvirt_storage = "/var/lib/libvirt/images"
libvirt_save = "/var/lib/libvirt/qemu/save"

[ipv6conf]
libvirt_interface_v6 = "virbr0-v6"
ipv6_bridge_mac = "52:54:00:12:34:56"
ipv6_prefix = "fd00::"
wan_interface = "eth0"
"#;

        let config: HustoaVmConfig = toml::from_str(config_str).unwrap();
        let ipv6conf = config.ipv6conf.unwrap();
        assert_eq!(ipv6conf.libvirt_interface_v6, "virbr0-v6");
        assert_eq!(ipv6conf.ipv6_bridge_mac, "52:54:00:12:34:56");
        assert_eq!(ipv6conf.wan_interface, "eth0");
        assert_eq!(ipv6conf.ipv6_prefix, Ipv6Addr::from_str("fd00::").unwrap());
    }

    #[test]
    fn test_config_pathbuf_types() {
        let config_str = r#"
[common]
libvirt_storage = "/var/lib/libvirt/images"
libvirt_save = "/var/lib/libvirt/qemu/save"
"#;

        let config: HustoaVmConfig = toml::from_str(config_str).unwrap();
        assert_eq!(config.common.libvirt_storage.as_path(), PathBuf::from("/var/lib/libvirt/images").as_path());
        assert!(config.common.libvirt_storage.is_absolute());
        assert_eq!(config.common.libvirt_save.as_path(), PathBuf::from("/var/lib/libvirt/qemu/save").as_path());
    }

    #[test]
    fn test_disk_size_variations() {
        let test_cases = vec![20, 40, 80, 120, 200];
        for size in test_cases {
            let config_str = format!(r#"
[common]
libvirt_storage = "/var/lib/libvirt/images"
libvirt_save = "/var/lib/libvirt/qemu/save"
default_disk_size = {}
"#, size);
            let config: HustoaVmConfig = toml::from_str(&config_str).unwrap();
            assert_eq!(config.common.default_disk_size, size);
        }
    }

    #[test]
    fn test_vcpus_variations() {
        let test_cases = vec![1, 2, 4, 8, 16, 32, 64];
        for vcpus in test_cases {
            let config_str = format!(r#"
[common]
libvirt_storage = "/var/lib/libvirt/images"
libvirt_save = "/var/lib/libvirt/qemu/save"
default_vcpus = {}
"#, vcpus);
            let config: HustoaVmConfig = toml::from_str(&config_str).unwrap();
            assert_eq!(config.common.default_vcpus, vcpus);
        }
    }

    #[test]
    fn test_memory_size_variations() {
        let test_cases = vec![1, 2, 4, 8, 16, 32, 64];
        for memory in test_cases {
            let config_str = format!(r#"
[common]
libvirt_storage = "/var/lib/libvirt/images"
libvirt_save = "/var/lib/libvirt/qemu/save"
default_memory_size = {}
"#, memory);
            let config: HustoaVmConfig = toml::from_str(&config_str).unwrap();
            assert_eq!(config.common.default_memory_size, memory);
        }
    }

    #[test]
    fn test_network_name_variations() {
        let networks = vec!["default", "mynetwork", "isolated", "bridge-net"];
        for network in networks {
            let config_str = format!(r#"
[common]
libvirt_storage = "/var/lib/libvirt/images"
libvirt_save = "/var/lib/libvirt/qemu/save"
libvirt_network = "{}"
"#, network);
            let config: HustoaVmConfig = toml::from_str(&config_str).unwrap();
            assert_eq!(config.common.libvirt_network, network);
        }
    }
}
