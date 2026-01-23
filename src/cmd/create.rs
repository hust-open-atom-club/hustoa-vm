use std::fs::remove_file;
use std::io::Write;
use std::net::Ipv6Addr;
use std::{error::Error, path::PathBuf};
use clap::Args;
use log::{debug, info, error};
use tempfile::NamedTempFile;
use slugify::slugify;
use serde::Serialize;
use serde_yaml;
use std::{fs, vec};
use crate::config::{global_config, HustoaVmConfig};
use crate::tools::{self, hustoa_run_cmd};
use crate::distro_info;
use crate::v6pool::V6Pool;

use super::MainCommandsRun;

#[derive(Args)]
pub struct SubCmdCreate {
    /// Name of the virtual machine
    #[arg(short, long)]
    name: String,

    /// Default user name, default to the vm's name
    #[arg(short, long)]
    user: Option<String>,

    /// Path of the ssh pubkey file
    #[arg(long)]
    ssh_pubkey: PathBuf,

    /// Distribution name, see `hustoa-vm distro`
    #[arg(short, long)]
    distro: String,

    /// Distribution version, default to the latest
    #[arg(long)]
    distro_version: Option<String>,

    /// Disk size, in GB
    #[arg(long, default_value_t = default_disk_size_arg())]
    disk_size: usize,

    /// Memory size, in GB
    #[arg(short, long, default_value_t = default_memory_size_arg())]
    memory: usize,

    /// Number of vcpus
    #[arg(long, default_value_t = default_vcpus_arg())]
    vcpus: usize,

    #[arg(long, default_value_t = false)]
    dryrun: bool,
}

fn default_disk_size_arg() -> usize {
    global_config.common.default_disk_size
}

fn default_memory_size_arg() -> usize {
    global_config.common.default_memory_size
}

fn default_vcpus_arg() -> usize {
    global_config.common.default_vcpus
}

#[derive(Debug)]
struct NewVmInfo {
    vm_name: String,
    host_name: String,
    download_link: String,
    virt_inst_osinfo: String,
    userdata_conf: String,
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
        let name_strip_space = slugify!(&args.name).replace("-", "_");

        let vm_name = format!("hustoa-vm-{}-{}-{}", name_strip_space, args.distro, rand_postfix);
        let user_name = match &args.user {
            Some(user) => slugify!(&user).replace("-", "_"),
            None => name_strip_space.clone(),
        };

        let host_name = format!("{}-{}-{}", name_strip_space, args.distro, rand_postfix);

        let ssh_pubkey = fs::read_to_string(&args.ssh_pubkey)?;

        let distro = args.distro.clone();
        let distro_info = distro_info::get_distro(&distro)?;
        let distro_version = match &args.distro_version {
            Some(version) => {
                distro_info.check_version(version)?
            },
            None => {
                distro_info.latest_version()
            }
        };
        info!("Selecting distro: {}, version: {}", distro, distro_version);
        let download_link = distro_info.get_download_link(&distro_version)?;
        let virt_inst_osinfo = distro_info.get_osinfo_conf(&distro_version)?;
        let userdata_conf = distro_info.gen_cloud_user_data(&distro_version, &user_name, &ssh_pubkey)?;

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
            host_name,
            download_link,
            virt_inst_osinfo,
            userdata_conf,
            interface,
            disk_path,
            seed_path,
            ipv6info,
        })
    }

    fn add_ndp_proxy(&self, config: &HustoaVmConfig) -> Result<(), Box<dyn Error>> {
        if let Some(ipv6conf) = &self.ipv6info {
            let mut pool = V6Pool::get_pool()?;
            pool.insert(&ipv6conf.v6_addr, &self.vm_name)?;
            pool.flush(config)?;
        }
        Ok(())
    }

    fn prepare_disk(&self, args: &SubCmdCreate) -> Result<(), Box<dyn Error>> {
        info!("Downloading disk file {}", self.download_link);
        let wget_res = hustoa_run_cmd("wget", [
                "-O",
                self.disk_path.to_str().unwrap(),
                &self.download_link
            ], args.dryrun).status()?;
        if wget_res.success() {
            info!("Download complete");
        } else {
            error!("wget download failed");
            return Err("wget error".into());
        }

        let qemu_img_res = hustoa_run_cmd("qemu-img", [
                "resize",
                self.disk_path.to_str().unwrap(),
                &format!("{}G", args.disk_size)
            ], args.dryrun).status()?;

        if !qemu_img_res.success() {
            error!("Resize image failed");
            return Err("qemu-img error".into());
        }

        Ok(())
    }

    fn prepare_cloud_init_files(&self, args: &SubCmdCreate) -> Result<(), Box<dyn Error>> {
        let mut userdata_config = NamedTempFile::new()?;
        let mut metadata_config = NamedTempFile::new()?;
        let mut network_config = NamedTempFile::new()?;

        userdata_config.write(self.userdata_conf.as_bytes())?;
        metadata_config.write(gen_meta_data_config(self).as_bytes())?;
        network_config.write(gen_network_config(self).as_bytes())?;

        let cloud_localds_res = hustoa_run_cmd("cloud-localds", [
                "-N",
                network_config.path().to_str().unwrap(),
                "-d",
                "qcow2",
                self.seed_path.to_str().unwrap(),
                userdata_config.path().to_str().unwrap(),
                metadata_config.path().to_str().unwrap(),
            ], args.dryrun).status()?;

        if !cloud_localds_res.success() {
            error!("Generate seed image failed");
            return Err("cloud-localds error".into());
        }
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
        let install_res = hustoa_run_cmd("virt-install", params, args.dryrun).status()?;

        if !install_res.success() {
            error!("Installation failed");
            return Err("virt-install error".into());
        }

        if !args.dryrun {
            std::fs::remove_file(&self.seed_path)?;
        }

        Ok(())
    }
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
    routes: Option<Vec<Route>>
}

#[derive(Debug, Serialize)]
struct Route {
    to: String,
    via: Ipv6Addr,

    #[serde(rename = "on-link")]
    on_link: bool
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
                routes: Some(vec![Route {
                    to: "default".to_string(),
                    via: ipv6info.v6_gateway,
                    on_link: true
                }]),
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
                    routes: None
                },
                enp2s0: enp2s0_config
            }
        }
    };
    let res = serde_yaml::to_string(&config).expect("cannot generate network config");
    "#cloud-config\n".to_string() + &res
}

impl SubCmdCreate {
    fn do_create(&self, vminfo: &NewVmInfo, config: &HustoaVmConfig) -> Result<(), Box<dyn Error>> {
        info!("Creating machine {}", vminfo.vm_name);
        vminfo.add_ndp_proxy(&config)?;

        info!("Preparing disk");
        vminfo.prepare_disk(self)?;

        info!("Preparing cloud init files");
        vminfo.prepare_cloud_init_files(self)?;

        info!("Perform vm installation");
        vminfo.install(self)?;

        info!("Installation complete");
        if let Some(ipv6info) = &vminfo.ipv6info {
            info!("Ipv6 address: {}", ipv6info.v6_addr);
        }
        Ok(())
    }
}

impl MainCommandsRun for SubCmdCreate {
    fn run_cmd(&self, config: &HustoaVmConfig) -> Result<(), Box<dyn Error>> {
        let vminfo = NewVmInfo::gen_new_vm_info(self, &config)?;
        debug!("get vm info: {:?}", vminfo);

        match self.do_create(&vminfo, config) {
            Ok(_) => Ok(()),
            Err(err) => {
                remove_file(vminfo.disk_path).ok();
                remove_file(vminfo.seed_path).ok();
                error!("Error when creating vm");
                Err(err)
            }
        }
    }
}
