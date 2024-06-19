mod cmd;
mod config;
mod tools;
mod v6pool;

use clap::{Parser, Subcommand};
use cmd::create::SubCmdCreate;
use cmd::v6pool::SubCmdV6Pool;
use env_logger;
use std::env;
use std::error::Error;
use log::error;
use config::HustoaVmConfig;

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a virtual machine
    Create(SubCmdCreate),

    /// Flush the ipv6 ndp proxy configuration
    V6Pool(SubCmdV6Pool),
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
    let config = match HustoaVmConfig::get_global_config() {
        Ok(conf) => conf,
        Err(msg) => {
            error!("{}", msg);
            return Err(msg)
        }
    };

    match &cli.command {
        Some(Commands::Create(args)) => {
            cmd::create::run_cmd(args, config)?;
        },
        Some(Commands::V6Pool(args)) => {
            cmd::v6pool::run_cmd(args, config)?;
        },
        None => {
            error!("Unsupported command.");
            return Err("Unsupported command".into())
        }
    }

    Ok(())
}
