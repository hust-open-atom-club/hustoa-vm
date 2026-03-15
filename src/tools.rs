use std::{ffi::OsStr, fmt::{Debug, Display}, net::Ipv6Addr};
use rand::{self, Rng};
use std::error::Error;
use semver::Version;
use std::process::Command;
use log::{debug, error, info};
use std::time::{Duration, Instant};
use std::thread::sleep;
use std::fs;
use std::path::Path;

pub fn hustoa_run_cmd<I, S>(program: S, args: I, dryrun: bool)
    -> Command
    where I: IntoIterator<Item = S> + Debug,
    S: AsRef<OsStr> + Display, {

    if let Err(_) = which::which(&program) {
        error!("Cannot find {} in PATH", program);
    }
    let mut cmd;
    if dryrun {
        info!("dryrun command {}, args: {:?}", program, args);
        cmd = Command::new("echo");
    } else {
        debug!("running command {}, args: {:?}", program, args);
        cmd = Command::new(program);
        cmd.args(args);
    }
    return cmd;
}

pub fn gen_mac_address_qemu() -> String {
    let mut rng = rand::thread_rng();
    let octets: Vec<String> = (0..3)
        .map(|_| format!("{:02x}", rng.gen_range(0..=255)))
        .collect();
    let postfix = octets.join(":");
    "52:54:00:".to_string() + &postfix
}

pub fn generate_eui64_from_mac(mac: &str, ipv6_prefix: Ipv6Addr) -> Result<Ipv6Addr, Box<dyn Error>> {
    // Split the MAC address into bytes
    let bytes: Vec<u8> = mac.split(':')
        .map(|s| u8::from_str_radix(s, 16))
        .collect::<Result<Vec<u8>, _>>()
        .map_err(|_| "Invalid MAC address format")?;

    if bytes.len() != 6 {
        return Err("MAC address must be 6 bytes".into());
    }

    // Create the EUI-64 by inserting `0xfffe` in the middle
    let mut eui64 = [0u8; 8];
    eui64[0..3].copy_from_slice(&bytes[0..3]);
    eui64[3..5].copy_from_slice(&[0xff, 0xfe]);
    eui64[5..8].copy_from_slice(&bytes[3..6]);

    // Flip the 7th bit of the first byte
    eui64[0] ^= 0x02;

    // Convert to Ipv6Addr format
    let ipv6_addr = Ipv6Addr::new(
        ipv6_prefix.segments()[0],
        ipv6_prefix.segments()[1],
        ipv6_prefix.segments()[2],
        ipv6_prefix.segments()[3],
        (eui64[0] as u16) << 8 | eui64[1] as u16,
        (eui64[2] as u16) << 8 | eui64[3] as u16,
        (eui64[4] as u16) << 8 | eui64[5] as u16,
        (eui64[6] as u16) << 8 | eui64[7] as u16,
    );

    Ok(ipv6_addr)
}

pub fn gen_rand_postfix() -> String {
    let mut bytes = vec![0u8; 4];
    rand::thread_rng().fill(&mut bytes[..]);
    let rand_posfix = hex::encode(bytes);
    rand_posfix.to_string()
}

pub fn virt_install_has_osinfo() -> bool {
    let res = hustoa_run_cmd("virt-install", ["--version"], false).output();
    match res {
        Ok(output) => {
            let version_now = String::from_utf8(output.stdout).unwrap();
            let version_now = version_now.replace("\n", "");
            let version_now = Version::parse(&version_now).unwrap();
            let min_version = Version::parse("3.0.0").unwrap();
            if min_version > version_now {
                false
            } else {
                true
            }
        },
        Err(msg) => {
            error!("{}", msg);
            false
        }
    }
}

// Try to resolve the IPv4 address for a VM by MAC address using libvirt DHCP leases.
// network_name is the libvirt network that provides DHCP (e.g. "default").
// Returns Some("192.168.122.100") on success or None on timeout.
pub fn resolve_ip_via_dhcp_leases(iface_mac: &str, network_name: &str, timeout_secs: u64) -> Option<String> {
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(timeout_secs) {
        let out = Command::new("virsh").arg("net-dhcp-leases").arg(network_name).output();
        if let Ok(output) = out {
            let s = String::from_utf8_lossy(&output.stdout).to_string();
            for line in s.lines() {
                if line.contains(iface_mac) {
                    // attempt to find an IPv4 address in the line
                    for token in line.split_whitespace() {
                        if token.contains('.') {
                            // token could be like 192.168.122.100/24
                            let ip = token.split('/').next().unwrap_or(token).to_string();
                            return Some(ip);
                        }
                    }
                }
            }
        }
        sleep(Duration::from_secs(2));
    }
    None
}

// Append or update VM entry in /etc/hustoa-vm/vmlist.toml
pub fn save_vm_entry_vmlist(name: &str, user: &str, distro: &str, disk_path: &str, ipv4: &str) -> Result<(), Box<dyn std::error::Error>> {
    let dir = Path::new("/etc/hustoa-vm");
    if !dir.exists() {
        fs::create_dir_all(dir)?;
    }
    let file_path = dir.join("vmlist.toml");

    // Read existing file into a toml value map
    let mut table = if file_path.exists() {
        let s = fs::read_to_string(&file_path)?;
        toml::from_str::<toml::Value>(&s)?
    } else {
        toml::Value::Table(toml::map::Map::new())
    };

    // prepare entry
    let mut entry = toml::map::Map::new();
    entry.insert("name".to_string(), toml::Value::String(name.to_string()));
    entry.insert("user".to_string(), toml::Value::String(user.to_string()));
    entry.insert("distro".to_string(), toml::Value::String(distro.to_string()));
    entry.insert("disk_path".to_string(), toml::Value::String(disk_path.to_string()));
    entry.insert("ipv4addr".to_string(), toml::Value::String(ipv4.to_string()));

    // use vm name as key
    if let toml::Value::Table(ref mut t) = table {
        t.insert(name.to_string(), toml::Value::Table(entry));
    }

    let out = toml::to_string_pretty(&table)?;
    fs::write(&file_path, out)?;
    Ok(())
}
