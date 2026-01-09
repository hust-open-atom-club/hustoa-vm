use clap::Args;
use std::error::Error;
use crate::{config::HustoaVmConfig, tools::hustoa_run_cmd};

use super::MainCommandsRun;

#[derive(Args)]
pub struct SubCmdSaveAll {
    #[arg(short, long)]
    dryrun: bool
}

impl MainCommandsRun for SubCmdSaveAll {
    fn run_cmd(&self, config: &HustoaVmConfig) -> Result<(), Box<dyn Error>> {
        let virsh_list = hustoa_run_cmd("virsh", ["list", "--name"], false).output()?;

        let running = String::from_utf8(virsh_list.stdout)?;
        let mut running: Vec<String> = running.lines().map(|x| x.to_string()).collect();
        running.retain(|x| *x != "");

        for vm in running {
            let save_path = config.common.libvirt_save.join(&vm);
            hustoa_run_cmd("virsh", ["save", vm.as_str(), save_path.to_str().unwrap()], self.dryrun).output()?;
        }

        Ok(())
    }
}
