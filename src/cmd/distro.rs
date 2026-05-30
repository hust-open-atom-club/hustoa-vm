use std::error::Error;

use clap::Args;

use crate::{config::HustoaVmConfig, distro_info::distro_version};
use colored::*;

use super::MainCommandsRun;

#[derive(Args)]
pub struct SubCmdDistro;

impl MainCommandsRun for SubCmdDistro {
    fn run_cmd(&self, _config: &HustoaVmConfig) -> Result<(), Box<dyn Error>> {
        for distro in &distro_version.distro {
            println!("Distro name: {}", distro.name.green());
            println!("Supported versions:");
            for version in &distro.versions {
                print!("\t{}", version.name);
                if version.alias.len() > 0 {
                    print!(" [alias: {}]", version.alias.join(", "));
                }
                println!()
            }
            println!()
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sub_cmd_distro_exists() {
        let _cmd = SubCmdDistro;
    }

    #[test]
    fn test_distro_version_list_not_empty() {
        assert!(!distro_version.distro.is_empty(), "Distro list should not be empty");
    }

    #[test]
    fn test_distro_version_has_required_fields() {
        for distro in &distro_version.distro {
            assert!(!distro.name.is_empty(), "Distro should have a name");
            assert!(!distro.latest_version.is_empty(), "Distro should have a latest version");
            assert!(!distro.versions.is_empty(), "Distro should have versions");
        }
    }

    #[test]
    fn test_distro_versions_have_required_fields() {
        for distro in &distro_version.distro {
            for version in &distro.versions {
                assert!(!version.name.is_empty(), "Version should have a name");
                assert!(version.alias.len() >= 0, "Version should have alias array");
            }
        }
    }
}
