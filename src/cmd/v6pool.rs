use std::{error::Error, net::Ipv6Addr};
use clap::{Args, Subcommand};
use crate::config::HustoaVmConfig;
use log::error;
use crate::v6pool::V6Pool;

#[derive(Args)]
pub struct SubCmdV6Pool {
    // Flush(CmdFlush),
    // Add(CmdAdd),
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Reload the ipv6 configuration
    Flush(CmdFlush),

    /// Add a new ipv6 address to the manager
    Add(CmdAdd),

    /// Delete ipv6 address
    Delete(CmdDelete),
}

#[derive(Args)]
struct CmdFlush {
}

#[derive(Args)]
struct CmdAdd {
    addr: Ipv6Addr
}

#[derive(Args)]
struct CmdDelete {
    addr: Ipv6Addr
}

fn run_cmd_flush(_args: &CmdFlush, config: HustoaVmConfig) -> Result<(), Box<dyn Error>> {
    let pool = V6Pool::get_pool()?;
    pool.flush(&config)?;
    Ok(())
}

fn run_cmd_add(args: &CmdAdd, config: HustoaVmConfig) -> Result<(), Box<dyn Error>> {
    let mut pool = V6Pool::get_pool()?;
    pool.insert(args.addr)?;
    pool.flush(&config)?;
    Ok(())
}

fn run_cmd_delete(args: &CmdDelete, _config: HustoaVmConfig) -> Result<(), Box<dyn Error>> {
    let mut pool = V6Pool::get_pool()?;
    pool.remove(args.addr)?;
    Ok(())
}

pub fn run_cmd(args: &SubCmdV6Pool, config: HustoaVmConfig) -> Result<(), Box<dyn Error>> {
    match &args.command {
        Some(Commands::Flush(subargs)) => run_cmd_flush(subargs, config)?,
        Some(Commands::Add(subargs)) => run_cmd_add(subargs, config)?,
        Some(Commands::Delete(subargs)) => run_cmd_delete(subargs, config)?,
        None => {
            error!("Unsupported command.");
            return Err("Unsupported command".into())
        }
    };

    Ok(())
}
