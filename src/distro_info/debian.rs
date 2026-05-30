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

        Ok(String::from(format!("https://cloud.debian.org/images/cloud/{}/latest/debian-{}-generic-{}.qcow2", item.name, item.alias[0], arch)))
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
        let res = serde_yml::to_string(&config).expect("cannot generate user config");
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debian_info_name() {
        let ubuntu_info = get_distro("ubuntu").unwrap();
        let debian_info = get_distro("debian").unwrap();
        assert_eq!(debian_info.name(), "debian");
        assert_ne!(debian_info.name(), ubuntu_info.name());
    }

    #[test]
    fn test_debian_latest_version() {
        let debian_info = get_distro("debian").unwrap();
        let latest = debian_info.latest_version();
        assert!(!latest.is_empty(), "Latest version should not be empty");
    }

    #[test]
    fn test_debian_check_version_valid() {
        let debian_info = get_distro("debian").unwrap();
        let valid_versions = vec!["12", "bookworm", "11", "bullseye"];
        for version in &valid_versions {
            let result = debian_info.check_version(&version.to_string());
            if result.is_ok() {
                assert!(!result.unwrap().is_empty());
            }
        }
    }

    #[test]
    fn test_debian_check_version_invalid() {
        let debian_info = get_distro("debian").unwrap();
        let result = debian_info.check_version(&"99.99".to_string());
        assert!(result.is_err(), "Invalid version should return error");
    }

    #[test]
    fn test_debian_get_osinfo_conf() {
        let debian_info = get_distro("debian").unwrap();
        let result = debian_info.get_osinfo_conf(&"12".to_string());
        assert!(result.is_ok(), "Should get osinfo conf for Debian 12");
        let osinfo = result.unwrap();
        assert!(!osinfo.is_empty(), "osinfo conf should not be empty");
    }

    #[test]
    fn test_debian_gen_cloud_user_data() {
        let debian_info = get_distro("debian").unwrap();
        let user = "testuser".to_string();
        let pubkey = "ssh-rsa AAAAB test@host".to_string();
        let version = "12".to_string();

        let result = debian_info.gen_cloud_user_data(&version, &user, &pubkey);
        assert!(result.is_ok(), "Should generate cloud user data");
        let userdata = result.unwrap();
        assert!(userdata.contains("#cloud-config"), "Should start with cloud-config");
        assert!(userdata.contains(&user), "Should contain username");
        assert!(userdata.contains(&pubkey), "Should contain pubkey");
    }

    #[test]
    fn test_debian_userdata_has_sudo() {
        let debian_info = get_distro("debian").unwrap();
        let user = "testuser".to_string();
        let pubkey = "ssh-rsa AAAAB test@host".to_string();
        let version = "12".to_string();

        let result = debian_info.gen_cloud_user_data(&version, &user, &pubkey);
        assert!(result.is_ok(), "Should generate cloud user data");
        let userdata = result.unwrap();
        assert!(userdata.contains("NOPASSWD"), "Should have sudo without password");
    }

    #[test]
    fn test_debian_userdata_has_shell() {
        let debian_info = get_distro("debian").unwrap();
        let user = "testuser".to_string();
        let pubkey = "ssh-rsa AAAAB test@host".to_string();
        let version = "12".to_string();

        let result = debian_info.gen_cloud_user_data(&version, &user, &pubkey);
        assert!(result.is_ok(), "Should generate cloud user data");
        let userdata = result.unwrap();
        assert!(userdata.contains("/bin/bash"), "Should have bash shell");
    }

    #[test]
    fn test_get_arch_codename_x86_64() {
        #[cfg(target_arch = "x86_64")]
        {
            let arch = get_arch_codename();
            assert_eq!(arch, Some("amd64".to_string()));
        }
    }

    #[test]
    fn test_get_arch_codename_aarch64() {
        #[cfg(target_arch = "aarch64")]
        {
            let arch = get_arch_codename();
            assert_eq!(arch, Some("arm64".to_string()));
        }
    }
}
