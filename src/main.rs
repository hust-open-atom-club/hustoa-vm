mod cmd;
mod config;
mod tools;
mod v6pool;
mod distro_info;

use clap::{Parser, Subcommand};
use cmd::create::SubCmdCreate;
use cmd::distro::SubCmdDistro;
use cmd::restore_all::SubCmdRestoreAll;
use cmd::save_all::SubCmdSaveAll;
use cmd::v6pool::SubCmdV6Pool;
use cmd::MainCommandsRun;
use enum_dispatch::enum_dispatch;
use env_logger;
use std::env;
use std::error::Error;
use log::error;
use config::{global_config, HustoaVmConfig};

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Option<MainCommands>,
}

#[derive(Subcommand)]
#[enum_dispatch]
enum MainCommands {
    /// Create a virtual machine
    Create(SubCmdCreate),

    /// Flush the ipv6 ndp proxy configuration
    V6Pool(SubCmdV6Pool),

    /// List the supported distributions
    Distro(SubCmdDistro),

    /// Save all running virsh vms (not only hustoa-vm)
    SaveAll(SubCmdSaveAll),

    /// Restore all running virsh vms (not only hustoa-vm)
    RestoreAll(SubCmdRestoreAll),
}

fn init_env_logger() {
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info")
    }

    env_logger::builder()
        .format_target(false)
        .format_timestamp(None)
        .init();
}

fn main() -> Result<(), Box<dyn Error>> {
    init_env_logger();
    let cli = Cli::parse();

    let config_res: &Result<HustoaVmConfig, Box<dyn Error + Send + Sync>> = &global_config;
    let config = match config_res {
        Ok(config) => config,
        Err(err) => {
            error!("{err}");
            return Err("Error on parse config file".into());
        }
    };

    match cli.command {
        Some(args) => args.run_cmd(&config)?,
        None => {
            error!("Unsupported command.");
            return Err("argument parser failed".into())
        }
    }
    Ok(())
}
