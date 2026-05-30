mod ubuntu;
mod debian;
mod archlinux;

use std::error::Error;
use archlinux::ArchlinuxInfo;
use debian::DebianInfo;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use ubuntu::UbuntuInfo;

pub trait Distro {
    /// Get the name of a distribution.
    #[allow(unused)]
    fn name(&self) -> String;

    fn check_version(&self, version: &String) -> Result<String, Box<dyn Error>>;

    fn latest_version(&self) -> String;

    fn get_download_link(&self, version: &String) -> Result<String, Box<dyn Error>>;

    fn get_osinfo_conf(&self, version: &String) -> Result<String, Box<dyn Error>>;

    fn gen_cloud_user_data(&self, version: &String, user: &String, pubkey: &String) -> Result<String, Box<dyn Error>>;
}

#[derive(Debug, Clone, Deserialize)]
pub struct DistroInfoList {
    pub distro: Vec<DistroInfo>
}

#[derive(Debug, Clone, Deserialize)]
pub struct DistroInfo {
    pub name: String,
    pub latest_version: String,
    pub versions: Vec<VersionInfo>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VersionInfo {
    pub name: String,
    pub alias: Vec<String>,
    pub osinfo_conf: String,
}

#[derive(Debug, Serialize)]
struct UserDataConfig {
    system_info: SystemInfo,

    #[serde(skip_serializing_if = "Option::is_none")]
    apt: Option<APTConfig>,

    #[serde(skip_serializing_if = "Option::is_none")]
    runcmd: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    cloud_config_modules: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    cloud_final_modules: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
struct SystemInfo {
    default_user: DefaultUser
}

#[derive(Debug, Serialize)]
struct DefaultUser {
    name: String,
    ssh_authorized_keys: Vec<String>,
    sudo: String,
    shell: String
}

#[derive(Debug, Serialize)]
struct APTConfig {
    primary: Vec<SourceConfig>,
    security: Vec<SourceConfig>,
}

#[derive(Debug, Serialize)]
struct SourceConfig {
    arches: Vec<String>,
    uri: String,
}

pub fn get_distro(name: &str) -> Result<Box<dyn Distro>, Box<dyn Error>> {
    let mut info: Option<&DistroInfo> = None;
    for distro in &distro_version.distro {
        if distro.name == name {
            info = Some(distro);
            break;
        }
    }
    if info.is_none() {
        return Err("Unsupported distro".into());
    }
    match name {
        "ubuntu" => Ok(Box::new(UbuntuInfo::new(info.unwrap()))),
        "debian" => Ok(Box::new(DebianInfo::new(info.unwrap()))),
        "archlinux" => Ok(Box::new(ArchlinuxInfo::new())),
        _ => return Err("Unsupported distro".into())
    }
}

fn get_version(info: &DistroInfo, version: &String) -> Result<VersionInfo, Box<dyn Error>> {
    for item in &info.versions {
        if item.name == *version {
            return Ok(item.clone());
        }
        if item.alias.contains(version) {
            return Ok(item.clone());
        }
    }
    Err(format!("Cannot found version {}", version).into())
}

lazy_static! {
    pub static ref distro_version: DistroInfoList = {
        let toml_str = include_str!("version_info.toml");
        let info: DistroInfoList = toml::from_str(toml_str).expect("init version_info failed");
        info
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_distro_ubuntu() {
        let distro = get_distro("ubuntu");
        assert!(distro.is_ok(), "Should get Ubuntu distro");
        let ubuntu = distro.unwrap();
        assert_eq!(ubuntu.name(), "ubuntu");
    }

    #[test]
    fn test_get_distro_debian() {
        let distro = get_distro("debian");
        assert!(distro.is_ok(), "Should get Debian distro");
        let debian = distro.unwrap();
        assert_eq!(debian.name(), "debian");
    }

    #[test]
    fn test_get_distro_archlinux() {
        let distro = get_distro("archlinux");
        assert!(distro.is_ok(), "Should get ArchLinux distro");
        let arch = distro.unwrap();
        assert_eq!(arch.name(), "archlinux");
    }

    #[test]
    fn test_get_distro_unsupported() {
        let distro = get_distro("fedora");
        assert!(distro.is_err(), "Should error on unsupported distro");
    }

    #[test]
    fn test_distro_info_list_not_empty() {
        assert!(!distro_version.distro.is_empty(), "Distro list should not be empty");
    }

    #[test]
    fn test_ubuntu_in_distro_list() {
        let ubuntu_found = distro_version.distro.iter().any(|d| d.name == "ubuntu");
        assert!(ubuntu_found, "Ubuntu should be in distro list");
    }

    #[test]
    fn test_debian_in_distro_list() {
        let debian_found = distro_version.distro.iter().any(|d| d.name == "debian");
        assert!(debian_found, "Debian should be in distro list");
    }

    #[test]
    fn test_all_distros_have_latest_version() {
        for distro in &distro_version.distro {
            assert!(!distro.latest_version.is_empty(), "{} should have latest_version", distro.name);
        }
    }

    #[test]
    fn test_all_distros_have_versions() {
        for distro in &distro_version.distro {
            assert!(!distro.versions.is_empty(), "{} should have versions", distro.name);
        }
    }

    #[test]
    fn test_all_versions_have_name() {
        for distro in &distro_version.distro {
            for version in &distro.versions {
                assert!(!version.name.is_empty(), "Version should have a name");
            }
        }
    }

    #[test]
    fn test_ubuntu_versions_have_osinfo() {
        let ubuntu = distro_version.distro.iter().find(|d| d.name == "ubuntu").unwrap();
        for version in &ubuntu.versions {
            assert!(!version.osinfo_conf.is_empty(), "Ubuntu version {} should have osinfo_conf", version.name);
        }
    }

    #[test]
    fn test_debian_versions_have_osinfo() {
        let debian = distro_version.distro.iter().find(|d| d.name == "debian").unwrap();
        for version in &debian.versions {
            assert!(!version.osinfo_conf.is_empty(), "Debian version {} should have osinfo_conf", version.name);
        }
    }

    #[test]
    fn test_get_version_valid() {
        let ubuntu = distro_version.distro.iter().find(|d| d.name == "ubuntu").unwrap();
        let version = get_version(ubuntu, &"22.04".to_string());
        assert!(version.is_ok(), "Should find valid version");
    }

    #[test]
    fn test_get_version_by_alias() {
        let ubuntu = distro_version.distro.iter().find(|d| d.name == "ubuntu").unwrap();
        let version = get_version(ubuntu, &"jammy".to_string());
        assert!(version.is_ok(), "Should find version by alias");
        // When using alias, the returned name is the canonical name
        assert!(version.unwrap().alias.contains(&"22.04".to_string()), "Version should have 22.04 as alias");
    }

    #[test]
    fn test_get_version_invalid() {
        let ubuntu = distro_version.distro.iter().find(|d| d.name == "ubuntu").unwrap();
        let version = get_version(ubuntu, &"99.99".to_string());
        assert!(version.is_err(), "Should error on invalid version");
    }
}
