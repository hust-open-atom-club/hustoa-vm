use std::{ffi::OsStr, fmt::{Debug, Display}, net::Ipv6Addr};
use rand::{self, Rng};
use std::error::Error;
use semver::Version;
use std::process::Command;
use log::{debug, error, info};

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
