use std::{error::Error, fs};
use clap::Args;
use crate::config::HustoaVmConfig;
use super::MainCommandsRun;
use log::info;
use std::process::Command;

#[derive(Args)]
#[clap(trailing_var_arg = true)]
pub struct SubCmdHook {
    /// Domain name passed from libvirt
    pub domain: Option<String>,

    /// Action passed from libvirt (e.g. prepare, release)
    pub action: Option<String>,

    /// Capture and ignore any extra arguments libvirt may pass (e.g. begin/end, "-")
    /// This prevents clap from erroring on unexpected trailing args.
    pub _extra: Vec<String>,
}

impl MainCommandsRun for SubCmdHook {
    fn run_cmd(&self, _config: &HustoaVmConfig) -> Result<(), Box<dyn Error>> {
        let domain = match &self.domain {
            Some(d) => d.clone(),
            None => return Ok(()),
        };
        let action = self.action.clone().unwrap_or_default();

        // Only handle prepare and release
        if action != "prepare" && action != "release" {
            return Ok(());
        }

        // Read vmlist
        let file = "/etc/hustoa-vm/vmlist.toml";
        let s = match fs::read_to_string(file) {
            Ok(x) => x,
            Err(_) => return Ok(()),
        };
        let tbl: toml::Value = toml::from_str(&s)?;

        let entry = match tbl.get(&domain) {
            Some(e) => e,
            None => return Ok(()),
        };

        let ip = entry.get("ipv4addr").and_then(|v| v.as_str()).unwrap_or("");
        if ip.is_empty() {
            return Ok(());
        }

        if let Some(ports) = entry.get("ports") {
            if let Some(arr) = ports.as_array() {
                for item in arr {
                    if let Some(host) = item.get("host").and_then(|v| v.as_integer()) {
                        if let Some(guest) = item.get("guest").and_then(|v| v.as_integer()) {
                            let host_s = host.to_string();
                            let guest_s = guest.to_string();
                            if action == "prepare" {
                                info!("Adding DNAT {} -> {}:{} (chain HUSTOA_VM)", host_s, ip, guest_s);
                                // ensure chain exists
                                let _ = Command::new("iptables").args(["-t", "nat", "-N", "HUSTOA_VM"]).status();
                                // ensure PREROUTING jumps to chain
                                let jump_exists = Command::new("iptables").args(["-t", "nat", "-C", "PREROUTING", "-j", "HUSTOA_VM"]).status()
                                    .map(|s| s.success()).unwrap_or(false);
                                if !jump_exists {
                                    let _ = Command::new("iptables").args(["-t", "nat", "-I", "PREROUTING", "-j", "HUSTOA_VM"]).status();
                                }

                                let _ = Command::new("iptables").args([
                                    "-t", "nat", "-A", "HUSTOA_VM", "-p", "tcp", "--dport", &host_s,
                                    "-j", "DNAT", "--to-destination", &format!("{}:{}", ip, guest_s),
                                ]).status();
                                // ensure forward rule allows traffic to guest
                                let forward_check = Command::new("iptables").args([
                                    "-C", "FORWARD", "-d", &ip, "-p", "tcp", "--dport", &guest_s, "-j", "ACCEPT"
                                ]).status().map(|s| s.success()).unwrap_or(false);
                                if !forward_check {
                                    let _ = Command::new("iptables").args([
                                        "-I", "FORWARD", "-d", &ip, "-p", "tcp", "--dport", &guest_s, "-j", "ACCEPT"
                                    ]).status();
                                }
                            } else if action == "release" {
                                info!("Removing DNAT {} -> {}:{} (chain HUSTOA_VM)", host_s, ip, guest_s);
                                let _ = Command::new("iptables").args([
                                    "-t", "nat", "-D", "HUSTOA_VM", "-p", "tcp", "--dport", &host_s,
                                    "-j", "DNAT", "--to-destination", &format!("{}:{}", ip, guest_s),
                                ]).status();
                                // remove forward accept rule
                                let _ = Command::new("iptables").args([
                                    "-D", "FORWARD", "-d", &ip, "-p", "tcp", "--dport", &guest_s, "-j", "ACCEPT"
                                ]).status();
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
