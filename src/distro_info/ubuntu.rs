use std::error::Error;
use crate::distro_info::*;

#[derive(Copy, Clone)]
pub struct UbuntuInfo<'a> {
    info: &'a DistroInfo
}

impl<'a> Distro for UbuntuInfo<'a> {
    fn name(&self) -> String {
        "ubuntu".to_string()
    }

    fn check_version(&self, version: &String) -> Result<String, Box<dyn Error>> {
        let item = self.get_version(version)?;
        Ok(item.name.into())
    }

    fn latest_version(&self) -> String {
        self.info.latest_version.clone()
    }

    fn get_download_link(&self, version: &String) -> Result<String, Box<dyn Error>> {
        let item = self.get_version(version)?;
        let arch = match get_arch_codename() {
            Some(arch) => arch,
            None => return Err("unsupport arch".into())
        };

        Ok(String::from(format!(
            "https://mirrors.ustc.edu.cn/ubuntu-cloud-images/{0}/current/{0}-server-cloudimg-{1}.img",
            item.name, arch)))
    }

    fn get_osinfo_conf(&self, version: &String) -> Result<String, Box<dyn Error>> {
        let item = self.get_version(version)?;
        Ok(item.osinfo_conf.to_string())
    }

    fn gen_cloud_user_data(&self, _version: &String, user: &String, pubkey: &String) -> Result<String, Box<dyn Error>> {
        let apt = self.gen_package_manager_config();
        let config = UserDataConfig {
            system_info: SystemInfo {
                default_user: DefaultUser {
                    name: user.clone(),
                    ssh_authorized_keys: vec![pubkey.clone()],
                    sudo: "ALL=(ALL) NOPASSWD:ALL".to_string(),
                    shell: "/bin/bash".to_string()
                }
            },
            apt
        };
        let res = serde_yaml::to_string(&config).expect("cannot generate user config");
        Ok("#cloud-config\n".to_string() + &res)
    }
}

impl<'a> UbuntuInfo<'a> {
    pub fn new() -> Self {
        for distro in &distro_version.distro {
            if distro.name == "ubuntu" {
                return Self {
                    info: &distro
                }
            }
        }
        panic!("ubuntu info not found")
    }

    fn gen_package_manager_config(&self) -> Option<APTConfig> {
        match std::env::consts::ARCH {
            "x86_64" => {
                Some(APTConfig {
                    primary: vec![SourceConfig {
                        arches: vec!["default".to_string()],
                        uri: "http://mirrors.hust.edu.cn/ubuntu".to_string(),
                    }],
                    security: vec![SourceConfig {
                        arches: vec!["default".to_string()],
                        uri: "http://security.ubuntu.com/ubuntu".to_string(),
                    }],
                })
            },
            "aarch64" => {
                Some(APTConfig {
                    primary: vec![SourceConfig {
                        arches: vec!["default".to_string()],
                        uri: "http://mirrors.ustc.edu.cn/ubuntu-ports".to_string(),
                    }],
                    security: vec![SourceConfig {
                        arches: vec!["default".to_string()],
                        uri: "http://ports.ubuntu.com/ubuntu-ports".to_string(),
                    }],
                })
            },
            _ => None
        }
    }

    fn get_version(&self, version: &String) -> Result<VersionInfo, Box<dyn Error>> {

        for item in &self.info.versions {
            if item.name == *version {
                return Ok(item.clone());
            }
            if item.alias.contains(version) {
                return Ok(item.clone());
            }
        }
        Err("Cannot found version".into())
    }
}

fn get_arch_codename() -> Option<String> {
    match std::env::consts::ARCH {
        "x86_64" => Some("amd64".to_string()),
        "aarch64" => Some("arm64".to_string()),
        _ => None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_ubuntu_info() {
        let ubuntu_info = get_distro("ubuntu").unwrap();
        assert_eq!(ubuntu_info.latest_version(), "noble");
        assert_eq!(ubuntu_info.get_osinfo_conf(&"focal".to_string()).unwrap(), "ubuntufocal");
        println!("{}", ubuntu_info.get_download_link(&"24.04".to_string()).unwrap());
    }
}
