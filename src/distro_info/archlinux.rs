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
        let res = serde_yaml::to_string(&config).expect("cannot generate user config");
        Ok("#cloud-config\n".to_string() + &res)
    }
}

impl ArchlinuxInfo {
    pub fn new() -> ArchlinuxInfo {
        ArchlinuxInfo
    }
}
