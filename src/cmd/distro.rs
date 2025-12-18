use std::error::Error;

use clap::Args;

use crate::{config::HustoaVmConfig, distro_info::distro_version};
use colored::*;

#[derive(Args)]
pub struct SubCmdDistro {
}

pub fn run_cmd(args: &SubCmdDistro, config: HustoaVmConfig) -> Result<(), Box<dyn Error>> {
    let _ = args;
    let _ = config;

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
    }
    Ok(())
}
