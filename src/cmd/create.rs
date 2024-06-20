use std::net::Ipv6Addr;
use std::{error::Error, path::PathBuf};
use clap::Args;
use log::{debug, info, error};
use std::process::Command;
use filenamify::filenamify;
use serde::Serialize;
use serde_yaml;
use std::{fs, vec};
use crate::config::HustoaVmConfig;
use crate::tools;
use crate::v6pool::V6Pool;

#[derive(Args)]
pub struct SubCmdCreate {
    /// Name of the virtual machine
    #[arg(short, long)]
    name: String,

    /// Path of the ssh pubkey file
    #[arg(long)]
    ssh_pubkey: PathBuf,

    /// Distribution name, supported: ubuntu
    #[arg(short, long)]
    distro: String,

    /// Distribution version, default to the latest
    #[arg(long)]
    distro_version: Option<String>,

    /// Disk size, in GB, default to 60
    #[arg(long, default_value_t = 60)]
    disk_size: u64,

    /// Memory size, in GB, default to 16
    #[arg(short, long, default_value_t = 16)]
    memory: u64,

    /// Number of vcpus, default to 16
    #[arg(long, default_value_t = 16)]
    vcpus: usize
}

#[derive(Debug)]
struct NewVmInfo {
    vm_name: String,
    user_name: String,
    host_name: String,
    distro: String,
    distro_version: String,
    ssh_pubkey: String,
    virt_inst_osinfo: String,
    interface: String,
    disk_path: PathBuf,
    seed_path: PathBuf,

    ipv6info: Option<Ipv6Info>
}

#[derive(Debug)]
struct Ipv6Info {
    v6_interface: String,
    v6_net_mac: String,
    v6_gateway: Ipv6Addr,
    v6_addr: Ipv6Addr,
}

impl NewVmInfo {
    fn gen_new_vm_info(args: &SubCmdCreate, config: &HustoaVmConfig) -> Result<NewVmInfo, Box<dyn Error>> {
        let rand_postfix = tools::gen_rand_postfix();
        let name_strip_space = filenamify(&args.name).replace(" ", "_");

        let vm_name = format!("hustoa-vm-{}-{}-{}", name_strip_space, args.distro, rand_postfix);
        let user_name = args.name.clone();
        let host_name = format!("{}-{}-{}", name_strip_space, args.distro, rand_postfix);

        let distro = args.distro.clone();
        let distro_version = args.distro_version.clone().unwrap_or_else(|| {
            match distro.as_str() {
                "ubuntu" => "noble".to_string(),
                "debian" => "bookworm".to_string(),
                "archlinux" => "".to_string(),
                _ => {
                    error!("Unsupport distribution");
                    "".to_string()
                }
            }
        });
        let virt_inst_osinfo = get_virt_inst_osinfo(&distro, &distro_version);

        let ssh_pubkey = fs::read_to_string(&args.ssh_pubkey)?;

        let disk_name = format!("{}.img", vm_name);
        let disk_path = config.common.libvirt_storage.join(disk_name);
        let seed_name = format!("seed-{}.img", vm_name);
        let seed_path = config.common.libvirt_storage.join(seed_name);
        let interface = config.common.libvirt_interface.clone();

        let mut ipv6info: Option<Ipv6Info> = None;

        match &config.ipv6conf {
            Some(ipv6conf) => {
                let mac = tools::gen_mac_address_qemu();
                ipv6info = Some(Ipv6Info {
                    v6_net_mac: mac.clone(),
                    v6_interface: ipv6conf.libvirt_interface_v6.clone(),
                    v6_gateway: tools::generate_eui64_from_mac(&ipv6conf.ipv6_bridge_mac,
                        Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 0))?,
                    v6_addr: tools::generate_eui64_from_mac(&mac, ipv6conf.ipv6_prefix)?
                })
            },
            None => {}
        }

        Ok(NewVmInfo {
            vm_name,
            user_name,
            host_name,
            distro,
            distro_version,
            ssh_pubkey,
            virt_inst_osinfo,
            interface,
            disk_path,
            seed_path,
            ipv6info,
        })
    }

    fn add_ndp_proxy(&self, config: &HustoaVmConfig) -> Result<(), Box<dyn Error>> {
        if let Some(ipv6conf) = &self.ipv6info {
            let mut pool = V6Pool::get_pool()?;
            pool.insert(ipv6conf.v6_addr)?;
            pool.flush(config)?;
        }
        Ok(())
    }

    fn prepare_disk(&self, args: &SubCmdCreate) -> Result<(), Box<dyn Error>> {
        let link = match get_download_link(&args.distro, &self.distro_version.clone()) {
            Some(link_str) => link_str,
            None => return Err("Download error".into())
        };

        info!("Downloading disk file {}", link);
        let wget_res = Command::new("wget")
            .args([
                "-O",
                self.disk_path.to_str().unwrap(),
                &link
            ])
            .status()?;
        if wget_res.success() {
            info!("Download complete");
        } else {
            error!("wget download failed");
            return Err("wget error".into());
        }

        let qemu_img_res = Command::new("qemu-img")
            .args([
                "resize",
                self.disk_path.to_str().unwrap(),
                &format!("{}G", args.disk_size)
            ])
            .status()?;

        if !qemu_img_res.success() {
            error!("Resize image failed");
            return Err("qemu-img error".into());
        }

        Ok(())
    }

    fn prepare_cloud_init_files(&self) -> Result<(), Box<dyn Error>> {
        let tmp_dir = PathBuf::from("/tmp");
        let userdata_config = tmp_dir.join(format!("userdata-{}.yaml", self.vm_name));
        let metadata_config = tmp_dir.join(format!("metadata-{}.yaml", self.vm_name));
        let network_config = tmp_dir.join(format!("network-{}.yaml", self.vm_name));

        fs::write(&userdata_config, gen_user_data_config(self))?;
        fs::write(&metadata_config, gen_meta_data_config(self))?;
        fs::write(&network_config, gen_network_config(self))?;

        let cloud_localds_res = Command::new("cloud-localds")
            .args([
                "-N",
                network_config.to_str().unwrap(),
                "-d",
                "qcow2",
                self.seed_path.to_str().unwrap(),
                userdata_config.to_str().unwrap(),
                metadata_config.to_str().unwrap(),
            ]).status()?;

        if !cloud_localds_res.success() {
            error!("Generate seed image failed");
            return Err("cloud-localds error".into());
        }

        fs::remove_file(userdata_config)?;
        fs::remove_file(metadata_config)?;
        fs::remove_file(network_config)?;
        Ok(())
    }

    fn install(&self, args: &SubCmdCreate) -> Result<(), Box<dyn Error>> {
        let has_osinfo = tools::virt_install_has_osinfo();
        let memory_in_mb = args.memory * 1024;
        let memory_in_mb = memory_in_mb.to_string();
        let vcpus = args.vcpus.to_string();
        let network_conf1 = format!("bridge={}", self.interface);
        let network_conf2;

        let mut params = vec![
            "--connect",
            "qemu:///system",
            "--import",
            "--memory",
            &memory_in_mb,
            "--vcpus",
            &vcpus,
            "--graphic",
            "none",
            "--name",
            &self.vm_name,
            "--disk",
            self.disk_path.to_str().unwrap(),
            "--cdrom",
            self.seed_path.to_str().unwrap(),
            "--network",
            &network_conf1,
        ];

        if let Some(ipv6info) = &self.ipv6info {
            network_conf2 = format!("bridge={},mac={}",
                ipv6info.v6_interface,
                ipv6info.v6_net_mac);

            params.push("--network");
            params.push(&network_conf2);
        }

        if has_osinfo {
            params.push("--osinfo");
            params.push(&self.virt_inst_osinfo);
        }

        debug!("virt-install params: {:?}", params);
        let install_res = Command::new("virt-install")
            .args(params)
            .status()?;

        if !install_res.success() {
            error!("Installation failed");
            return Err("virt-install error".into());
        }

        std::fs::remove_file(&self.seed_path)?;

        Ok(())
    }
}

fn get_arch_codename() -> Option<String> {
    match std::env::consts::ARCH {
        "x86_64" => Some("amd64".to_string()),
        "aarch64" => Some("arm64".to_string()),
        _ => None
    }
}

fn get_virt_inst_osinfo(distro: &String, distro_version: &String) -> String {
    match distro.as_str() {
        "ubuntu" => "ubuntu-stable-latest".to_string(),
        "debian" => format!("debian{}", distro_version),
        _ => "linux2022".to_string()
    }
}

fn get_download_link_ubuntu(version: &String) -> Option<String> {
    Some(String::from(format!(
        "https://mirrors.ustc.edu.cn/ubuntu-cloud-images/{0}/current/{0}-server-cloudimg-{1}.img",
        version, get_arch_codename()?)))
    }

fn get_download_link_debian(_version: &String) -> Option<String> {
    todo!("fix the link");
    // return Some(String::from(format!(
    //     "https://mirrors.ustc.edu.cn/debian-cdimage/cloud/{0}/latest/debian-12-generic-{1}.qcow2",
    //     version, get_arch_codename()?)));
}

fn get_download_link_archlinux(_version: &String) -> Option<String> {
    None
}

fn get_download_link(distro: &String, version: &String) -> Option<String> {
    match distro.as_str() {
        "ubuntu" => get_download_link_ubuntu(version),
        "debian" => get_download_link_debian(version),
        "archlinux" => get_download_link_archlinux(version),
        _ => None
    }
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

fn gen_package_manager_config_ubuntu(_vminfo: &NewVmInfo) -> Option<APTConfig> {
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
}

fn gen_package_manager_config_debian(_vminfo: &NewVmInfo) -> Option<APTConfig> {
    Some(APTConfig {
        primary: vec![SourceConfig {
            arches: vec!["default".to_string()],
            uri: "http://mirrors.hust.edu.cn/debian".to_string(),
        }],
        security: vec![SourceConfig {
            arches: vec!["default".to_string()],
            uri: "https://security.debian.org/debian-security".to_string(),
        }],
    })
}

fn gen_package_manager_config(vminfo: &NewVmInfo) -> Option<APTConfig> {
    match vminfo.distro.as_str() {
        "ubuntu" => gen_package_manager_config_ubuntu(vminfo),
        "debian" => gen_package_manager_config_debian(vminfo),
        _ => None
    }
}

fn gen_user_data_config(vminfo: &NewVmInfo) -> String {
    let apt = gen_package_manager_config(vminfo);

    let config = UserDataConfig {
        system_info: SystemInfo {
            default_user: DefaultUser {
                name: vminfo.user_name.clone(),
                ssh_authorized_keys: vec![vminfo.ssh_pubkey.clone()],
                sudo: "ALL=(ALL) NOPASSWD:ALL".to_string(),
                shell: "/bin/bash".to_string()
            }
        },
        apt
    };

    let res = serde_yaml::to_string(&config).expect("cannot generate user config");
    "#cloud-config\n".to_string() + &res
}

#[derive(Debug, Serialize)]
struct MetaDataConfig {
    #[serde(rename = "instance-id")]
    instance_id: String,

    #[serde(rename = "local-hostname")]
    local_hostname: String
}

fn gen_meta_data_config(vminfo: &NewVmInfo) -> String {
    let config = MetaDataConfig {
        instance_id: vminfo.vm_name.clone(),
        local_hostname: vminfo.host_name.clone(),
    };
    let res = serde_yaml::to_string(&config).expect("cannot generate meta data config");
    "#cloud-config\n".to_string() + &res
}

#[derive(Debug, Serialize)]
struct NetworkConfig {
    network: Network
}

#[derive(Debug, Serialize)]
struct Network {
    ethernets: Ethernets,
    version: u8
}
#[derive(Debug, Serialize)]
struct Ethernets {
    enp1s0: EthernetConfig,

    #[serde(skip_serializing_if = "Option::is_none")]
    enp2s0: Option<EthernetConfig>,
}

#[derive(Debug, Serialize)]
struct EthernetConfig {
    dhcp4: bool,
    dhcp6: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    addresses: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    gateway6: Option<Ipv6Addr>
}

fn gen_network_config(vminfo: &NewVmInfo) -> String {
    let enp2s0_config = match &vminfo.ipv6info {
        Some(ipv6info) => {
            let v6addr = ipv6info.v6_addr.to_string();
            let v6addr = v6addr + "/64";
            Some(EthernetConfig {
                dhcp4: false,
                dhcp6: false,
                addresses: Some(vec![v6addr]),
                gateway6: Some(ipv6info.v6_gateway),
            })
        },
        None => None
    };
    let config = NetworkConfig {
        network: Network {
            version: 2,
            ethernets: Ethernets {
                enp1s0: EthernetConfig {
                    dhcp4: true,
                    dhcp6: false,
                    addresses: None,
                    gateway6: None
                },
                enp2s0: enp2s0_config
            }
        }
    };
    let res = serde_yaml::to_string(&config).expect("cannot generate network config");
    "#cloud-config\n".to_string() + &res
}

pub fn run_cmd(args: &SubCmdCreate, config: HustoaVmConfig) -> Result<(), Box<dyn Error>> {
    let vminfo = NewVmInfo::gen_new_vm_info(args, &config)?;
    debug!("get vm info: {:?}", vminfo);
    info!("Creating machine {}", vminfo.vm_name);

    vminfo.add_ndp_proxy(&config)?;

    info!("Preparing disk");
    vminfo.prepare_disk(args)?;

    debug!("Preparing cloud init files");
    vminfo.prepare_cloud_init_files()?;

    info!("Perform vm installation");
    vminfo.install(args)?;

    info!("Installation complete");
    if let Some(ipv6info) = &vminfo.ipv6info {
        info!("Ipv6 address: {}", ipv6info.v6_addr);
    }
    Ok(())
}
