use std::error::Error;

use clap::Args;

use crate::{config::HustoaVmConfig, distro_info::distro_version};
use colored::*;

use super::MainCommandsRun;

#[derive(Args)]
pub struct SubCmdDistro;

impl MainCommandsRun for SubCmdDistro {
    fn run_cmd(&self, _config: &HustoaVmConfig) -> Result<(), Box<dyn Error>> {
        for distro in &distro_version.distro {
            println!("Distro name: {}", distro.name.green());
            println!("Supported versions:");
            for version in &distro.versions {
                print!("\t{}", version.name);
                if version.alias.len() > 0 {
                    print!(" [alias: {}]", version.alias.join(", "));
                }
                println!()
            }
            println!()
        }
        Ok(())
    }
}
