use std::{error::Error, fs, path::PathBuf, os::unix::fs::PermissionsExt};
use clap::Args;
use log::{info, error};
use tempfile::tempdir;

use crate::tools::hustoa_run_cmd;
use crate::config::HustoaVmConfig;

use super::MainCommandsRun;

/// Download the latest release tarball for the host architecture from GitHub,
/// extract the `hustoa-vm` binary and install it to `/usr/local/bin/hustoa-vm`.
///
/// Supported architectures: `aarch64`, `x86_64`.
#[derive(Args)]
pub struct SubCmdSelfUpdate {
    /// Optional architecture override (aarch64 | x86_64)
    #[arg(long)]
    pub arch: Option<String>,
}

impl MainCommandsRun for SubCmdSelfUpdate {
    fn run_cmd(&self, _config: &HustoaVmConfig) -> Result<(), Box<dyn Error>> {
        // determine architecture: use override if provided, otherwise detect host
        let arch = match &self.arch {
            Some(a) => a.clone(),
            None => std::env::consts::ARCH.to_string(),
        };

        // Map rust arch names to release file names.
        // Assumption: x86_64 release filename is `hustoa-vm-x86_64-unknown-linux-musl.tar.gz`.
        let filename = match arch.as_str() {
            "aarch64" => "hustoa-vm-aarch64-unknown-linux-musl.tar.gz",
            "x86_64" => "hustoa-vm-x86_64-unknown-linux-musl.tar.gz",
            other => {
                return Err(format!("Unsupported architecture: {}. Supported: aarch64, x86_64", other).into())
            }
        };

        let url = format!("https://github.com/hust-open-atom-club/hustoa-vm/releases/latest/download/{}", filename);
        info!("Downloading latest release from {}", url);

        let tmp = tempdir()?;
        let tar_path = tmp.path().join("hustoa-vm.tar.gz");
        let tar_path_str = tar_path.to_str().ok_or("invalid temp path")?;

        // Download using wget (keeps this toolchain consistent with other commands in the project).
        let wget_status = hustoa_run_cmd("wget", ["-O", tar_path_str, &url], false).status()?;
        if !wget_status.success() {
            error!("wget failed to download file");
            return Err("wget download failed".into());
        }

        // Extract tarball to temp dir
        let extract_status = hustoa_run_cmd("tar", ["-xzf", tar_path_str, "-C", tmp.path().to_str().unwrap()], false).status()?;
        if !extract_status.success() {
            error!("tar extraction failed");
            return Err("tar extraction failed".into());
        }

        let extracted_bin = tmp.path().join("hustoa-vm");
        if !extracted_bin.exists() {
            error!("extracted binary not found: {}", extracted_bin.display());
            return Err("extracted binary not found".into());
        }

        let target_path = PathBuf::from("/usr/local/bin/hustoa-vm");

        // Attempt to copy into /usr/local/bin and set executable permissions.
        // This will fail if the user doesn't have permission; that's expected behavior.
        match fs::copy(&extracted_bin, &target_path) {
            Ok(_) => {
                let mut perms = fs::metadata(&target_path)?.permissions();
                perms.set_mode(0o755);
                fs::set_permissions(&target_path, perms)?;
                info!("Installed hustoa-vm to {}", target_path.display());
            }
            Err(e) => {
                error!("Failed to install to {}: {}", target_path.display(), e);
                return Err(format!("Failed to install to {}: {}", target_path.display(), e).into());
            }
        }

        // tempdir is cleaned up automatically
        Ok(())
    }
}
