use std::{error::Error, net::Ipv6Addr, str::FromStr};
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
    /// List the managed ipv6 addresses
    List(CmdList),

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
struct CmdList;

impl V6PoolCommandsRun for CmdList {
    fn run_cmd(&self, _config: &HustoaVmConfig) -> Result<(),Box<dyn Error>> {
        let pool = V6Pool::get_pool()?;
        pool.print();
        Ok(())
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cmd_list_exists() {
        let _cmd = CmdList;
    }

    #[test]
    fn test_cmd_flush_exists() {
        let _cmd = CmdFlush;
    }

    #[test]
    fn test_cmd_purge_exists() {
        let _cmd = CmdPurge;
    }

    #[test]
    fn test_cmd_add_has_fields() {
        let addr = Ipv6Addr::from_str("2001:db8::1").unwrap();
        let domain = "test.com".to_string();
        let cmd = CmdAdd { addr, domain: domain.clone() };
        assert_eq!(cmd.addr, addr);
        assert_eq!(cmd.domain, domain);
    }

    #[test]
    fn test_cmd_delete_has_field() {
        let addr = Ipv6Addr::from_str("2001:db8::1").unwrap();
        let cmd = CmdDelete { addr };
        assert_eq!(cmd.addr, addr);
    }

    #[test]
    fn test_cmd_delete_by_name_has_field() {
        let name = "testvm".to_string();
        let cmd = CmdDeleteByName { name: name.clone() };
        assert_eq!(cmd.name, name);
    }

    #[test]
    fn test_cmd_generate_net_define_defaults() {
        let cmd = CmdGenerateNetDefine {
            name: "hustoa-netv6".to_string(),
            iface: "virbr6".to_string(),
        };
        assert_eq!(cmd.name, "hustoa-netv6");
        assert_eq!(cmd.iface, "virbr6");
    }

    #[test]
    fn test_cmd_generate_net_define_custom() {
        let cmd = CmdGenerateNetDefine {
            name: "custom-net".to_string(),
            iface: "custom-br".to_string(),
        };
        assert_eq!(cmd.name, "custom-net");
        assert_eq!(cmd.iface, "custom-br");
    }

    #[test]
    fn test_ipv6_addr_parsing() {
        let test_cases = vec![
            "2001:db8::1",
            "fd00::100",
            "fe80::1",
            "::1",
        ];
        for addr_str in test_cases {
            let addr = Ipv6Addr::from_str(addr_str);
            assert!(addr.is_ok(), "Should parse IPv6 address: {}", addr_str);
        }
    }

    #[test]
    fn test_ipv6_addr_invalid_parsing() {
        let test_cases = vec![
            "invalid",
            "192.168.1.1",
            "",
        ];
        for addr_str in test_cases {
            let addr = Ipv6Addr::from_str(addr_str);
            assert!(addr.is_err(), "Should fail to parse invalid IPv6: {}", addr_str);
        }
    }
}
