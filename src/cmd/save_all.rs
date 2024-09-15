use clap::Args;
use log::debug;
use std::{error::Error, process::Command};
use crate::config::HustoaVmConfig;

#[derive(Args)]
pub struct SubCmdSaveAll;

pub fn run_cmd(args: &SubCmdSaveAll, config: HustoaVmConfig) -> Result<(), Box<dyn Error>> {
    let virsh_list = Command::new("virsh")
        .args(["list", "--name"]).output()?;

    let running = String::from_utf8(virsh_list.stdout)?;
    let mut running: Vec<String> = running.lines().map(|x| x.to_string()).collect();
    running.retain(|x| *x != "");

    for vm in running {
        let save_path = config.common.libvirt_storage.join(&vm);
        let cmd = ["save", vm.as_str(), save_path.to_str().unwrap()];
        debug!("running virsh command: {:?}", cmd);
        Command::new("virsh").args(cmd).output()?;
    }

    Ok(())
}
