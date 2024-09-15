use clap::Args;
use std::{error::Error, fs};
use crate::{config::HustoaVmConfig, tools::hustoa_run_cmd};

use super::MainCommandsRun;

#[derive(Args)]
pub struct SubCmdRestoreAll {
    #[arg(short, long)]
    dryrun: bool
}

impl MainCommandsRun for SubCmdRestoreAll {
    fn run_cmd(&self, config: &HustoaVmConfig) -> Result<(), Box<dyn Error>> {
        for entry in fs::read_dir(&config.common.libvirt_save)? {
            let path = entry?.path();
            if path.is_file() {
                hustoa_run_cmd("virsh", ["restore", path.to_str().unwrap()], self.dryrun).output()?;
            }
        }

        Ok(())
    }
}
