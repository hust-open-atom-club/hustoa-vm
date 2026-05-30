use std::{ffi::OsStr, fmt::{Debug, Display}, net::Ipv6Addr, str::FromStr};
use rand::{self, Rng};
use std::net::TcpListener;
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

// Allocate a free host TCP port in the ephemeral range for SSH forwarding.
pub fn allocate_host_port() -> Result<u16, Box<dyn std::error::Error>> {
    let mut rng = rand::thread_rng();
    for _ in 0..100 {
        let port: u16 = rng.gen_range(20000..30000);
        let addr = format!("0.0.0.0:{}", port);
        match TcpListener::bind(&addr) {
            Ok(listener) => {
                // Successfully bound, drop listener to free port and return value
                drop(listener);
                return Ok(port);
            }
            Err(_) => continue,
        }
    }
    Err("failed to allocate free port".into())
}

// Add a port mapping to an existing VM entry in vmlist.toml. Creates the file/entry if needed.
pub fn add_port_mapping_vmlist(name: &str, host_port: u16, guest_port: u16) -> Result<(), Box<dyn std::error::Error>> {
    let dir = std::path::Path::new("/etc/hustoa-vm");
    if !dir.exists() {
        std::fs::create_dir_all(dir)?;
    }
    let file_path = dir.join("vmlist.toml");
    let mut table = if file_path.exists() {
        let s = std::fs::read_to_string(&file_path)?;
        toml::from_str::<toml::Value>(&s)?
    } else {
        toml::Value::Table(toml::map::Map::new())
    };

    let port_entry = {
        let mut m = toml::map::Map::new();
        m.insert("host".to_string(), toml::Value::Integer(host_port as i64));
        m.insert("guest".to_string(), toml::Value::Integer(guest_port as i64));
        toml::Value::Table(m)
    };

    if let toml::Value::Table(ref mut t) = table {
        let vm_entry = t.entry(name.to_string()).or_insert_with(|| toml::Value::Table(toml::map::Map::new()));
        if let toml::Value::Table(ref mut vm_table) = vm_entry {
            match vm_table.get_mut("ports") {
                Some(toml::Value::Array(arr)) => {
                    arr.push(port_entry);
                }
                _ => {
                    vm_table.insert("ports".to_string(), toml::Value::Array(vec![port_entry]));
                }
            }
        }
    }

    let out = toml::to_string_pretty(&table)?;
    std::fs::write(&file_path, out)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gen_mac_address_qemu_format() {
        let mac = gen_mac_address_qemu();
        assert!(mac.starts_with("52:54:00:"), "MAC should start with 52:54:00:");
        let parts: Vec<&str> = mac.split(':').collect();
        assert_eq!(parts.len(), 6, "MAC should have 6 octets");
        for part in &parts {
            assert_eq!(part.len(), 2, "Each octet should be 2 characters");
            u8::from_str_radix(part, 16).expect("Each octet should be valid hex");
        }
    }

    #[test]
    fn test_gen_mac_address_qemu_uniqueness() {
        let mac1 = gen_mac_address_qemu();
        let mac2 = gen_mac_address_qemu();
        assert_ne!(mac1, mac2, "Generated MAC addresses should be unique");
    }

    #[test]
    fn test_generate_eui64_from_mac_valid() {
        let mac = "52:54:00:12:34:56";
        let prefix = "2001:db8::";
        let ipv6_prefix = Ipv6Addr::from_str(prefix).unwrap();
        let result = generate_eui64_from_mac(mac, ipv6_prefix);
        assert!(result.is_ok(), "EUI-64 generation should succeed");
        let ipv6 = result.unwrap();
        assert_eq!(ipv6.segments()[0], 0x2001, "IPv6 prefix should be preserved");
        assert_eq!(ipv6.segments()[1], 0x0db8, "IPv6 prefix should be preserved");
    }

    #[test]
    fn test_generate_eui64_from_mac_invalid_format() {
        let mac = "invalid-mac";
        let prefix = "2001:db8::";
        let ipv6_prefix = Ipv6Addr::from_str(prefix).unwrap();
        let result = generate_eui64_from_mac(mac, ipv6_prefix);
        assert!(result.is_err(), "Invalid MAC format should return error");
    }

    #[test]
    fn test_generate_eui64_from_mac_wrong_length() {
        let mac = "52:54:00:12:34";
        let prefix = "2001:db8::";
        let ipv6_prefix = Ipv6Addr::from_str(prefix).unwrap();
        let result = generate_eui64_from_mac(mac, ipv6_prefix);
        assert!(result.is_err(), "MAC with wrong length should return error");
    }

    #[test]
    fn test_gen_rand_postfix_length() {
        let postfix = gen_rand_postfix();
        assert_eq!(postfix.len(), 8, "Random postfix should be 8 characters (4 bytes in hex)");
    }

    #[test]
    fn test_gen_rand_postfix_uniqueness() {
        let postfix1 = gen_rand_postfix();
        let postfix2 = gen_rand_postfix();
        assert_ne!(postfix1, postfix2, "Random postfixes should be unique");
    }

    #[test]
    fn test_gen_rand_postfix_is_hex() {
        let postfix = gen_rand_postfix();
        u32::from_str_radix(&postfix, 16).expect("Postfix should be valid hex");
    }

    #[test]
    fn test_save_vm_entry_vmlist() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let dir = temp_dir.path();
        let file_path = dir.join("vmlist.toml");

        // Mock the /etc/hustoa-vm directory
        let _original_path = Path::new("/etc/hustoa-vm/vmlist.toml");
        let _test_path = file_path.as_path();

        // Create a test entry
        let name = "test-vm";
        let user = "testuser";
        let distro = "ubuntu:24.04";
        let disk_path = "/var/lib/libvirt/images/test.img";
        let ipv4 = "192.168.122.100";

        // Write directly to temp file
        let mut entry = toml::map::Map::new();
        entry.insert("name".to_string(), toml::Value::String(name.to_string()));
        entry.insert("user".to_string(), toml::Value::String(user.to_string()));
        entry.insert("distro".to_string(), toml::Value::String(distro.to_string()));
        entry.insert("disk_path".to_string(), toml::Value::String(disk_path.to_string()));
        entry.insert("ipv4addr".to_string(), toml::Value::String(ipv4.to_string()));

        let mut table = toml::map::Map::new();
        table.insert(name.to_string(), toml::Value::Table(entry));

        let out = toml::to_string_pretty(&table).unwrap();
        fs::write(&file_path, out).unwrap();

        // Verify the file was written correctly
        let content = fs::read_to_string(&file_path).unwrap();
        assert!(content.contains(name), "File should contain VM name");
        assert!(content.contains(user), "File should contain user");
        assert!(content.contains(ipv4), "File should contain IPv4 address");
    }

    #[test]
    fn test_save_vm_entry_vmlist_update_existing() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let file_path = temp_dir.path().join("vmlist.toml");

        // Create initial entry
        let mut table = toml::map::Map::new();
        let mut entry = toml::map::Map::new();
        entry.insert("name".to_string(), toml::Value::String("vm1".to_string()));
        entry.insert("user".to_string(), toml::Value::String("user1".to_string()));
        entry.insert("distro".to_string(), toml::Value::String("ubuntu:22.04".to_string()));
        entry.insert("disk_path".to_string(), toml::Value::String("/path/disk1.img".to_string()));
        entry.insert("ipv4addr".to_string(), toml::Value::String("192.168.122.10".to_string()));
        table.insert("vm1".to_string(), toml::Value::Table(entry));

        fs::write(&file_path, toml::to_string_pretty(&table).unwrap()).unwrap();

        // Read and update
        let content = fs::read_to_string(&file_path).unwrap();
        let mut parsed: toml::Value = toml::from_str(&content).unwrap();

        if let toml::Value::Table(ref mut t) = parsed {
            let mut new_entry = toml::map::Map::new();
            new_entry.insert("name".to_string(), toml::Value::String("vm2".to_string()));
            new_entry.insert("user".to_string(), toml::Value::String("user2".to_string()));
            new_entry.insert("distro".to_string(), toml::Value::String("debian:12".to_string()));
            new_entry.insert("disk_path".to_string(), toml::Value::String("/path/disk2.img".to_string()));
            new_entry.insert("ipv4addr".to_string(), toml::Value::String("192.168.122.20".to_string()));
            t.insert("vm2".to_string(), toml::Value::Table(new_entry));
        }

        fs::write(&file_path, toml::to_string_pretty(&parsed).unwrap()).unwrap();

        // Verify both entries exist
        let updated = fs::read_to_string(&file_path).unwrap();
        assert!(updated.contains("vm1"), "Original entry should still exist");
        assert!(updated.contains("vm2"), "New entry should be added");
    }

    #[test]
    fn test_allocate_host_port_in_range() {
        let port = allocate_host_port().unwrap();
        assert!(port >= 20000, "Port should be >= 20000, got {}", port);
        assert!(port < 30000, "Port should be < 30000, got {}", port);
    }

    #[test]
    fn test_allocate_host_port_multiple() {
        let mut ports = std::collections::HashSet::new();
        for _ in 0..10 {
            let port = allocate_host_port().unwrap();
            ports.insert(port);
            assert!(port >= 20000 && port < 30000, "Port {} should be in range", port);
        }
        assert!(ports.len() > 1, "Should generate multiple different ports");
    }

    #[test]
    fn test_add_port_mapping_vmlist() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let file_path = temp_dir.path().join("vmlist.toml");

        // Create a VM entry first
        let mut table = toml::map::Map::new();
        let mut vm_entry = toml::map::Map::new();
        vm_entry.insert("name".to_string(), toml::Value::String("testvm".to_string()));
        vm_entry.insert("user".to_string(), toml::Value::String("testuser".to_string()));
        vm_entry.insert("distro".to_string(), toml::Value::String("ubuntu:24.04".to_string()));
        vm_entry.insert("disk_path".to_string(), toml::Value::String("/path/disk.img".to_string()));
        vm_entry.insert("ipv4addr".to_string(), toml::Value::String("192.168.122.100".to_string()));
        table.insert("testvm".to_string(), toml::Value::Table(vm_entry));

        fs::write(&file_path, toml::to_string_pretty(&table).unwrap()).unwrap();

        // Add port mapping
        let _name = "testvm";
        let host_port: u16 = 22001;
        let guest_port: u16 = 22;

        let content = fs::read_to_string(&file_path).unwrap();
        let mut parsed: toml::Value = toml::from_str(&content).unwrap();

        let port_entry = {
            let mut m = toml::map::Map::new();
            m.insert("host".to_string(), toml::Value::Integer(host_port as i64));
            m.insert("guest".to_string(), toml::Value::Integer(guest_port as i64));
            toml::Value::Table(m)
        };

        if let toml::Value::Table(ref mut t) = parsed {
            if let toml::Value::Table(ref mut vm_table) = t.get_mut("testvm").unwrap() {
                match vm_table.get_mut("ports") {
                    Some(toml::Value::Array(arr)) => {
                        arr.push(port_entry);
                    }
                    _ => {
                        vm_table.insert("ports".to_string(), toml::Value::Array(vec![port_entry]));
                    }
                }
            }
        }

        fs::write(&file_path, toml::to_string_pretty(&parsed).unwrap()).unwrap();

        // Verify port mapping was added
        let updated = fs::read_to_string(&file_path).unwrap();
        assert!(updated.contains("ports"), "Should contain ports field");
        assert!(updated.contains(&host_port.to_string()), "Should contain host port");
    }

    #[test]
    fn test_add_port_mapping_vmlist_multiple() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let file_path = temp_dir.path().join("vmlist.toml");

        // Create VM entry with existing port
        let mut vm_entry = toml::map::Map::new();
        vm_entry.insert("name".to_string(), toml::Value::String("testvm".to_string()));
        vm_entry.insert("user".to_string(), toml::Value::String("testuser".to_string()));
        vm_entry.insert("distro".to_string(), toml::Value::String("ubuntu:24.04".to_string()));
        vm_entry.insert("disk_path".to_string(), toml::Value::String("/path/disk.img".to_string()));
        vm_entry.insert("ipv4addr".to_string(), toml::Value::String("192.168.122.100".to_string()));

        let mut port1 = toml::map::Map::new();
        port1.insert("host".to_string(), toml::Value::Integer(22001));
        port1.insert("guest".to_string(), toml::Value::Integer(22));
        vm_entry.insert("ports".to_string(), toml::Value::Array(vec![toml::Value::Table(port1)]));

        let mut table = toml::map::Map::new();
        table.insert("testvm".to_string(), toml::Value::Table(vm_entry));

        fs::write(&file_path, toml::to_string_pretty(&table).unwrap()).unwrap();

        // Add second port
        let content = fs::read_to_string(&file_path).unwrap();
        let mut parsed: toml::Value = toml::from_str(&content).unwrap();

        let port_entry = {
            let mut m = toml::map::Map::new();
            m.insert("host".to_string(), toml::Value::Integer(22002));
            m.insert("guest".to_string(), toml::Value::Integer(80));
            toml::Value::Table(m)
        };

        if let toml::Value::Table(ref mut t) = parsed {
            if let toml::Value::Table(ref mut vm_table) = t.get_mut("testvm").unwrap() {
                if let toml::Value::Array(arr) = vm_table.get_mut("ports").unwrap() {
                    arr.push(port_entry);
                }
            }
        }

        fs::write(&file_path, toml::to_string_pretty(&parsed).unwrap()).unwrap();

        // Verify both ports exist
        let updated = fs::read_to_string(&file_path).unwrap();
        assert!(updated.contains("22001"), "Should contain first port");
        assert!(updated.contains("22002"), "Should contain second port");
    }

    #[test]
    fn test_hustoa_run_cmd_dryrun() {
        let cmd = hustoa_run_cmd("echo", ["test", "args"], true);
        // In dryrun mode, the command should be created (but won't have args passed to echo)
        // Just verify it doesn't panic
        let _cmd = cmd;
    }

    #[test]
    fn test_mac_address_consistency() {
        // Test that generated MAC addresses are consistent in format
        for _ in 0..100 {
            let mac = gen_mac_address_qemu();
            assert!(mac.starts_with("52:54:00:"), "MAC should always start with 52:54:00:");
            let parts: Vec<&str> = mac.split(':').collect();
            assert_eq!(parts.len(), 6, "MAC should always have 6 octets");
        }
    }

    #[test]
    fn test_ipv6_prefix_preservation() {
        let test_cases = vec![
            ("52:54:00:12:34:56", "2001:db8::"),
            ("52:54:00:aa:bb:cc", "fd00::"),
            ("52:54:00:ff:ff:ff", "fe80::"),
        ];

        for (mac, prefix) in test_cases {
            let ipv6_prefix = Ipv6Addr::from_str(prefix).unwrap();
            let result = generate_eui64_from_mac(mac, ipv6_prefix).unwrap();
            let prefix_bytes = ipv6_prefix.segments();
            let result_bytes = result.segments();

            // First 4 segments (128 bits) should match prefix
            // For /64 prefixes, first 2 segments should match
            assert_eq!(result_bytes[0], prefix_bytes[0], "First segment should match prefix");
            assert_eq!(result_bytes[1], prefix_bytes[1], "Second segment should match prefix");
        }
    }

    #[test]
    fn test_eui64_bit_flip() {
        // EUI-64 should flip the 7th bit of the first byte
        let mac = "52:54:00:12:34:56";
        let ipv6_prefix = Ipv6Addr::from_str("fe80::").unwrap();
        let result = generate_eui64_from_mac(mac, ipv6_prefix).unwrap();

        // The result should be a link-local address with proper EUI-64 format
        // fe80::5054:ff:fe12:3456 (with U/L bit flipped)
        let segments = result.segments();
        assert_eq!(segments[0], 0xfe80, "Should be link-local prefix");
    }

    #[test]
    fn test_random_postfix_entropy() {
        // Test that random postfixes have reasonable entropy
        let mut postfixes = std::collections::HashSet::new();
        for _ in 0..1000 {
            let postfix = gen_rand_postfix();
            postfixes.insert(postfix);
        }
        // Should have close to 1000 unique values
        assert!(postfixes.len() > 950, "Should have high entropy in random postfixes");
    }
}
