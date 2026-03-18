use std::error::Error;

use enum_dispatch::enum_dispatch;

use crate::config::HustoaVmConfig;

pub mod create;
pub mod v6pool;
pub mod distro;
pub mod save_all;
pub mod restore_all;
pub mod self_update;
pub mod install_hook;
pub mod hook;

#[enum_dispatch(MainCommands)]
pub trait MainCommandsRun {
    fn run_cmd(&self, config: &HustoaVmConfig) -> Result<(), Box<dyn Error>>;
}
