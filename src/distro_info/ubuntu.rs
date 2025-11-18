use std::error::Error;
use crate::distro_info::*;

#[derive(Copy, Clone)]
pub struct UbuntuInfo<'a> {
    valid_versions: &'a [Version<'a>],
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
        self.valid_versions[0].name.to_string()
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

    fn get_version(&self, version: &String) -> Result<Version, Box<dyn Error>> {
        for item in self.valid_versions {
            if item.name == version {
                return Ok(item.clone());
            }
            if item.alias.contains(&version.as_str()) {
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

#[allow(private_interfaces)]
pub static UBUNTU_INFO: UbuntuInfo = UbuntuInfo {
    valid_versions: &[
        Version {
            name: "noble",
            alias: &[
                "24.04"
            ],
            osinfo_conf: "ubuntu-stable-latest"
        },
        Version {
            name: "jammy",
            alias: &[
                "22.04"
            ],
            osinfo_conf: "ubuntujammy"
        },
        Version {
            name: "focal",
            alias: &[
                "20.04"
            ],
            osinfo_conf: "ubuntufocal"
        },
        Version {
            name: "bionic",
            alias: &[
                "18.04"
            ],
            osinfo_conf: "ubuntubionic"
        },
        // Version {
        //     name: "xenial",
        //     alias: &[
        //         "16.04"
        //     ],
        //     osinfo_conf: "ubuntuxenial"
        // },
        // Version {
        //     name: "trusty",
        //     alias: &[
        //         "14.04"
        //     ],
        //     osinfo_conf: "ubuntutrusty"
        // },
    ]
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_ubuntu_info() {
        assert_eq!(UBUNTU_INFO.latest_version(), "noble");
        assert_eq!(UBUNTU_INFO.get_osinfo_conf(&"focal".to_string()).unwrap(), "ubuntufocal");
        println!("{}", UBUNTU_INFO.get_download_link(&"24.04".to_string()).unwrap());
    }
}
