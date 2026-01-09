use std::{error::Error, net::Ipv6Addr};
use clap::{Args, Subcommand};
use enum_dispatch::enum_dispatch;
use crate::{config::HustoaVmConfig, tools::{gen_mac_address_qemu, generate_eui64_from_mac}};
use log::error;
use crate::v6pool::V6Pool;

use super::MainCommandsRun;

#[derive(Args)]
pub struct SubCmdV6Pool {
    // Flush(CmdFlush),
    // Add(CmdAdd),
    #[command(subcommand)]
    command: Option<V6PoolCommands>,
}

#[derive(Subcommand)]
#[enum_dispatch]
enum V6PoolCommands {
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

#[enum_dispatch(V6PoolCommands)]
trait V6PoolCommandsRun {
    fn run_cmd(&self, config: &HustoaVmConfig) -> Result<(), Box<dyn Error>>;
}


#[derive(Args)]
struct CmdFlush;

impl V6PoolCommandsRun for CmdFlush {
    fn run_cmd(&self, config: &HustoaVmConfig) -> Result<(), Box<dyn Error>> {
        let pool = V6Pool::get_pool()?;
        pool.flush(&config)?;
        Ok(())
    }
}

#[derive(Args)]
struct CmdAdd {
    /// IPv6 Address
    addr: Ipv6Addr,

    /// Domain name, will be used to delete unused entry
    domain: String,
}

impl V6PoolCommandsRun for CmdAdd {
    fn run_cmd(&self, config: &HustoaVmConfig) -> Result<(),Box<dyn Error>> {
        let mut pool = V6Pool::get_pool()?;
        pool.insert(&self.addr, &self.domain)?;
        pool.flush(&config)?;
        Ok(())
    }
}

#[derive(Args)]
struct CmdPurge;

impl V6PoolCommandsRun for CmdPurge {
    fn run_cmd(&self, config: &HustoaVmConfig) -> Result<(),Box<dyn Error>> {
        let mut pool = V6Pool::get_pool()?;

        pool.purge(&config)
    }
}

#[derive(Args)]
struct CmdDelete {
    /// IPv6 Address
    addr: Ipv6Addr
}

impl V6PoolCommandsRun for CmdDelete {
    fn run_cmd(&self, _config: &HustoaVmConfig) -> Result<(),Box<dyn Error>> {
        let mut pool = V6Pool::get_pool()?;
        pool.remove_by_addr(&self.addr)?;
        Ok(())
    }
}

#[derive(Args)]
struct CmdDeleteByName {
    /// Domain name
    name: String,
}

impl V6PoolCommandsRun for CmdDeleteByName {
    fn run_cmd(&self, _config: &HustoaVmConfig) -> Result<(),Box<dyn Error>> {
        let mut pool = V6Pool::get_pool()?;
        pool.remove_by_name(&self.name)?;
        Ok(())
    }
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

impl V6PoolCommandsRun for CmdGenerateNetDefine {

    fn run_cmd(&self, config: &HustoaVmConfig) -> Result<(),Box<dyn Error>> {
        let mac = gen_mac_address_qemu();
        let ipv6conf = match &config.ipv6conf {
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
        name_ = self.name,
        iface_ = self.iface,
        mac_ = mac,
        v6addr_ = ipv6_addr);

        println!("{}", xml);
        Ok(())
    }
}

impl MainCommandsRun for SubCmdV6Pool {
    fn run_cmd(&self, config: &HustoaVmConfig) -> Result<(), Box<dyn Error>> {
        match &self.command {
            Some(args) => args.run_cmd(config)?,
            None => {
                error!("Unsupported command.");
                return Err("V6pool Subcommand parser failed".into())
            }
        }
        Ok(())
    }
}
