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
