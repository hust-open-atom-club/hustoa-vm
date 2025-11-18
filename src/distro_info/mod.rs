mod ubuntu;

use std::error::Error;
use serde::Serialize;

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

pub fn get_distro(distro: &String) -> Result<Box<dyn Distro>, Box<dyn Error>> {
    match distro.as_str() {
        "ubuntu" => Ok(Box::new(ubuntu::UBUNTU_INFO)),
        _ => Err("unsupported distro".into())
    }
}

#[derive(Copy, Clone)]
struct Version<'a> {
    name: &'a str,
    alias: &'a [&'a str],
    osinfo_conf: &'a str,
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
