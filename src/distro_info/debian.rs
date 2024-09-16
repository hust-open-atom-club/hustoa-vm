use std::error::Error;
use crate::distro_info::*;

#[derive(Copy, Clone)]
pub struct DebianInfo<'a> {
    info: &'a DistroInfo
}

impl<'a> Distro for DebianInfo<'a> {
    fn name(&self) -> String {
        "debian".to_string()
    }

    fn check_version(&self, version: &String) -> Result<String, Box<dyn Error>> {
        let item = get_version(&self.info, version)?;
        Ok(item.name.into())
    }

    fn latest_version(&self) -> String {
        self.info.latest_version.clone()
    }

    fn get_download_link(&self, version: &String) -> Result<String, Box<dyn Error>> {
        let item = get_version(&self.info, version)?;
        let arch = match get_arch_codename() {
            Some(arch) => arch,
            None => return Err("unsupport arch".into())
        };

        Ok(String::from(format!("https://mirrors.ustc.edu.cn/debian-cdimage/cloud/{}/latest/debian-{}-generic-{}.qcow2", item.name, item.alias[0], arch)))
    }

    fn get_osinfo_conf(&self, version: &String) -> Result<String, Box<dyn Error>> {
        let item = get_version(self.info, version)?;
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
            apt,
            runcmd: None,
            cloud_config_modules: None,
            cloud_final_modules: None,
        };
        let res = serde_yaml::to_string(&config).expect("cannot generate user config");
        Ok("#cloud-config\n".to_string() + &res)
    }
}

impl<'a> DebianInfo<'a> {
    pub fn new(info: &'a DistroInfo) -> Self {
        Self {
            info
        }
    }

    fn gen_package_manager_config(&self) -> Option<APTConfig> {
        Some(APTConfig {
            primary: vec![SourceConfig {
                arches: vec!["default".to_string()],
                uri: "http://mirrors.hust.edu.cn/debian".to_string(),
            }],
            security: vec![SourceConfig {
                arches: vec!["default".to_string()],
                uri: "http://security.debian.org/debian".to_string(),
            }],
        })
    }
}

fn get_arch_codename() -> Option<String> {
    match std::env::consts::ARCH {
        "x86_64" => Some("amd64".to_string()),
        "aarch64" => Some("arm64".to_string()),
        _ => None
    }
}
