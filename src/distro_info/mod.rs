mod ubuntu;

use std::error::Error;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

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

#[derive(Clone, Deserialize)]
pub struct DistroInfoList {
    distro: Vec<DistroInfo>
}

#[derive(Clone, Deserialize)]
pub struct DistroInfo {
    name: String,
    latest_version: String,
    versions: Vec<VersionInfo>,
}

#[derive(Clone, Deserialize)]
pub struct VersionInfo {
    name: String,
    alias: Vec<String>,
    osinfo_conf: String,
}

#[derive(Debug, Serialize)]
struct UserDataConfig {
    system_info: SystemInfo,

    #[serde(skip_serializing_if = "Option::is_none")]
    apt: Option<APTConfig>,
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

pub fn get_distro(distro: &str) -> Result<Box<dyn Distro>, Box<dyn Error>> {
    match distro {
        "ubuntu" => Ok(Box::new(ubuntu::UbuntuInfo::new())),
        _ => Err("Unknown distribution name".into())
    }
}

lazy_static! {
    pub static ref distro_version: DistroInfoList = {
        let toml_str = include_str!("version_info.toml");
        let info: DistroInfoList = toml::from_str(toml_str).expect("init version_info failed");
        info
    };
}
