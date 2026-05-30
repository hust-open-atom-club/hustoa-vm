use crate::distro_info::*;

pub struct ArchlinuxInfo;

impl Distro for ArchlinuxInfo {
    fn name(&self) -> String {
        "archlinux".to_string()
    }

    fn check_version(&self, _version: &String) -> Result<String, Box<dyn Error>> {
        Ok("rolling".to_string())
    }

    fn latest_version(&self) -> String {
        "rolling".to_string()
    }

    fn get_download_link(&self, _version: &String) -> Result<String, Box<dyn Error>> {
        Ok("https://mirrors.hust.edu.cn/archlinux/images/latest/Arch-Linux-x86_64-cloudimg.qcow2".to_string())
    }

    fn get_osinfo_conf(&self, _version: &String) -> Result<String, Box<dyn Error>> {
        Ok("archlinux".to_string())
    }

    fn gen_cloud_user_data(&self, _version: &String, user: &String, pubkey: &String) -> Result<String, Box<dyn Error>> {
        let mirror = "'Server = http://mirrors.hust.edu.cn/archlinux/$repo/os/$arch'";
        let cmd_pacman_src = format!("echo {mirror} > /etc/pacman.d/mirrorlist");
        let cmd_update = format!("pacman -Sy");
        let cmd_enable = format!("systemctl enable sshd");
        let cmd_start = format!("systemctl start sshd");

        let config = UserDataConfig {
            system_info: SystemInfo {
                default_user: DefaultUser {
                    name: user.clone(),
                    ssh_authorized_keys: vec![pubkey.clone()],
                    sudo: "ALL=(ALL) NOPASSWD:ALL".to_string(),
                    shell: "/bin/bash".to_string()
                }
            },
            apt: None,
            runcmd: Some(vec![cmd_pacman_src, cmd_update, cmd_enable, cmd_start]),
            cloud_config_modules: Some(vec!["runcmd".to_string()]),
            cloud_final_modules: Some(vec!["scripts-user".to_string()]),
        };
        let res = serde_yml::to_string(&config).expect("cannot generate user config");
        Ok("#cloud-config\n".to_string() + &res)
    }
}

impl ArchlinuxInfo {
    pub fn new() -> ArchlinuxInfo {
        ArchlinuxInfo
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_archlinux_info_name() {
        let arch = ArchlinuxInfo::new();
        assert_eq!(arch.name(), "archlinux");
    }

    #[test]
    fn test_archlinux_latest_version() {
        let arch = ArchlinuxInfo::new();
        assert_eq!(arch.latest_version(), "rolling");
    }

    #[test]
    fn test_archlinux_check_version() {
        let arch = ArchlinuxInfo::new();
        let result = arch.check_version(&"rolling".to_string());
        assert!(result.is_ok(), "ArchLinux version check should succeed");
        assert_eq!(result.unwrap(), "rolling");
    }

    #[test]
    fn test_archlinux_check_version_any() {
        let arch = ArchlinuxInfo::new();
        let versions = vec!["rolling", "latest", "2024.01"];
        for version in &versions {
            let result = arch.check_version(&version.to_string());
            assert!(result.is_ok(), "Version check should succeed for {}", version);
        }
    }

    #[test]
    fn test_archlinux_get_download_link() {
        let arch = ArchlinuxInfo::new();
        let result = arch.get_download_link(&"rolling".to_string());
        assert!(result.is_ok(), "Should get download link");
        let link = result.unwrap();
        assert!(link.contains("archlinux"), "Link should contain archlinux");
        assert!(link.contains("cloudimg"), "Link should contain cloudimg");
    }

    #[test]
    fn test_archlinux_get_osinfo_conf() {
        let arch = ArchlinuxInfo::new();
        let result = arch.get_osinfo_conf(&"rolling".to_string());
        assert!(result.is_ok(), "Should get osinfo conf");
        assert_eq!(result.unwrap(), "archlinux");
    }

    #[test]
    fn test_archlinux_gen_cloud_user_data() {
        let arch = ArchlinuxInfo::new();
        let user = "testuser".to_string();
        let pubkey = "ssh-rsa AAAAB test@host".to_string();
        let version = "rolling".to_string();

        let result = arch.gen_cloud_user_data(&version, &user, &pubkey);
        assert!(result.is_ok(), "Should generate cloud user data");
        let userdata = result.unwrap();
        assert!(userdata.contains("#cloud-config"), "Should start with cloud-config");
        assert!(userdata.contains(&user), "Should contain username");
        assert!(userdata.contains(&pubkey), "Should contain pubkey");
    }

    #[test]
    fn test_archlinux_userdata_has_pacman_commands() {
        let arch = ArchlinuxInfo::new();
        let user = "testuser".to_string();
        let pubkey = "ssh-rsa AAAAB test@host".to_string();
        let version = "rolling".to_string();

        let result = arch.gen_cloud_user_data(&version, &user, &pubkey);
        assert!(result.is_ok(), "Should generate cloud user data");
        let userdata = result.unwrap();
        assert!(userdata.contains("pacman"), "Should have pacman commands");
        assert!(userdata.contains("mirrors.hust.edu.cn"), "Should use HUST mirrors");
    }

    #[test]
    fn test_archlinux_userdata_has_sshd_commands() {
        let arch = ArchlinuxInfo::new();
        let user = "testuser".to_string();
        let pubkey = "ssh-rsa AAAAB test@host".to_string();
        let version = "rolling".to_string();

        let result = arch.gen_cloud_user_data(&version, &user, &pubkey);
        assert!(result.is_ok(), "Should generate cloud user data");
        let userdata = result.unwrap();
        assert!(userdata.contains("sshd"), "Should have sshd commands");
        assert!(userdata.contains("systemctl"), "Should have systemctl commands");
    }

    #[test]
    fn test_archlinux_userdata_has_runcmd() {
        let arch = ArchlinuxInfo::new();
        let user = "testuser".to_string();
        let pubkey = "ssh-rsa AAAAB test@host".to_string();
        let version = "rolling".to_string();

        let result = arch.gen_cloud_user_data(&version, &user, &pubkey);
        assert!(result.is_ok(), "Should generate cloud user data");
        let userdata = result.unwrap();
        assert!(userdata.contains("runcmd"), "Should have runcmd section");
    }

    #[test]
    fn test_archlinux_userdata_has_sudo() {
        let arch = ArchlinuxInfo::new();
        let user = "testuser".to_string();
        let pubkey = "ssh-rsa AAAAB test@host".to_string();
        let version = "rolling".to_string();

        let result = arch.gen_cloud_user_data(&version, &user, &pubkey);
        assert!(result.is_ok(), "Should generate cloud user data");
        let userdata = result.unwrap();
        assert!(userdata.contains("NOPASSWD"), "Should have sudo without password");
    }

    #[test]
    fn test_archlinux_download_link_format() {
        let arch = ArchlinuxInfo::new();
        let result = arch.get_download_link(&"rolling".to_string());
        assert!(result.is_ok(), "Should get download link");
        let link = result.unwrap();
        assert!(link.starts_with("https://"), "Link should start with https://");
        assert!(link.ends_with(".qcow2"), "Link should end with .qcow2");
    }
}
