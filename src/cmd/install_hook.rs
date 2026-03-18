use std::{error::Error, fs, path::Path};
use std::os::unix::fs::PermissionsExt;
use clap::Args;
use crate::config::HustoaVmConfig;
use super::MainCommandsRun;
use log::info;

#[derive(Args)]
pub struct SubCmdInstallHook {}

impl MainCommandsRun for SubCmdInstallHook {
    fn run_cmd(&self, _config: &HustoaVmConfig) -> Result<(), Box<dyn Error>> {
        let hook_path = Path::new("/etc/libvirt/hooks/qemu");
        let wrapper = r#"#!/bin/sh
# Wrapper to delegate to hustoa-vm hook subcommand
exec /usr/local/bin/hustoa-vm hook "$@"
"#;

        if hook_path.exists() {
            // backup existing hook
            let backup = hook_path.with_extension("bak");
            fs::rename(&hook_path, &backup)?;
            info!("Existing hook backed up to {}", backup.display());
        } else {
            if let Some(parent) = hook_path.parent() {
                fs::create_dir_all(parent)?;
            }
        }

        fs::write(hook_path, wrapper)?;
        let mut perms = fs::metadata(hook_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(hook_path, perms)?;
        info!("Installed libvirt hook wrapper to {}", hook_path.display());
        Ok(())
    }
}
