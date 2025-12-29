use std::{error::Error, net::Ipv6Addr};
use clap::{Args, Subcommand};
use crate::{config::HustoaVmConfig, tools::{gen_mac_address_qemu, generate_eui64_from_mac}};
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

    /// Clean the unused ipv6 pool entry
    Purge(CmdPurge),

    /// Add a new ipv6 address to the manager
    Add(CmdAdd),

    /// Delete ipv6 address
    Delete(CmdDelete),

    /// Delete ipv6 address by domain name
    DeleteByName(CmdDeleteByName),

    /// Generate libvirt network define xml for ipv6
    GenV6NetXML(CmdGenerateNetDefine),
}

#[derive(Args)]
struct CmdFlush;

#[derive(Args)]
struct CmdAdd {
    /// IPv6 Address
    addr: Ipv6Addr,

    /// Domain name, will be used to delete unused entry
    domain: String,
}

#[derive(Args)]
struct CmdPurge;

#[derive(Args)]
struct CmdDelete {
    /// IPv6 Address
    addr: Ipv6Addr
}

#[derive(Args)]
struct CmdDeleteByName {
    /// Domain name
    name: String,
}

#[derive(Args)]
struct CmdGenerateNetDefine {
    #[arg(short, long, default_value_t = { "hustoa-netv6".to_string() })]
    /// Set libvirt network name
    name: String,

    #[arg(short, long, default_value_t = { "virbr6".to_string() })]
    /// Set bridge network interface name
    iface: String,

}


fn run_cmd_flush(_args: &CmdFlush, config: HustoaVmConfig) -> Result<(), Box<dyn Error>> {
    let pool = V6Pool::get_pool()?;
    pool.flush(&config)?;
    Ok(())
}

fn run_cmd_purge(_args: &CmdPurge, config: HustoaVmConfig) -> Result<(), Box<dyn Error>> {
    let mut pool = V6Pool::get_pool()?;

    pool.purge(&config)
}

fn run_cmd_add(args: &CmdAdd, config: HustoaVmConfig) -> Result<(), Box<dyn Error>> {
    let mut pool = V6Pool::get_pool()?;
    pool.insert(&args.addr, &args.domain)?;
    pool.flush(&config)?;
    Ok(())
}

fn run_cmd_delete(args: &CmdDelete, config: HustoaVmConfig) -> Result<(), Box<dyn Error>> {
    let _ = config;
    let mut pool = V6Pool::get_pool()?;
    pool.remove_by_addr(&args.addr)?;
    Ok(())
}

fn run_cmd_delete_by_name(args: &CmdDeleteByName, config: HustoaVmConfig) -> Result<(), Box<dyn Error>> {
    let _ = config;
    let mut pool = V6Pool::get_pool()?;
    pool.remove_by_name(&args.name)?;
    Ok(())
}

fn run_cmd_genv6netxml(args: &CmdGenerateNetDefine, config: HustoaVmConfig)
    -> Result<(), Box<dyn Error>> {

    let mac = gen_mac_address_qemu();
    let ipv6conf = match config.ipv6conf {
        Some(ipv6conf) => ipv6conf,
        None => return Err("no ipv6 config provided".into()),
    };
    let ipv6_addr = generate_eui64_from_mac(&mac, ipv6conf.ipv6_prefix)?;

    let xml = format!("<network>
  <name>{name_}</name>
  <forward mode='open'/>
  <bridge name='{iface_}' stp='on' delay='0'/>
  <mac address='{mac_}'/>
  <domain name='{name_}' localOnly='yes'/>
  <ip family='ipv6' address='{v6addr_}' prefix='64'>
  </ip>
</network>",
    name_ = args.name,
    iface_ = args.iface,
    mac_ = mac,
    v6addr_ = ipv6_addr);

    println!("{}", xml);
    Ok(())
}

pub fn run_cmd(args: &SubCmdV6Pool, config: HustoaVmConfig) -> Result<(), Box<dyn Error>> {
    match &args.command {
        Some(Commands::Flush(subargs)) => run_cmd_flush(subargs, config)?,
        Some(Commands::Purge(subargs)) => run_cmd_purge(subargs, config)?,
        Some(Commands::Add(subargs)) => run_cmd_add(subargs, config)?,
        Some(Commands::Delete(subargs)) => run_cmd_delete(subargs, config)?,
        Some(Commands::DeleteByName(subargs)) => run_cmd_delete_by_name(subargs, config)?,
        Some(Commands::GenV6NetXML(subargs)) => run_cmd_genv6netxml(subargs, config)?,
        None => {
            error!("Unsupported command.");
            return Err("Unsupported command".into())
        }
    };

    Ok(())
}
