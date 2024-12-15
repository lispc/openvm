use std::path::PathBuf;

use clap::Parser;
use eyre::Result;
use openvm_sdk::{
    config::{AppConfig, SdkVmConfig},
    fs::{write_app_pk_to_file, write_app_vk_to_file},
    Sdk,
};

use crate::{
    default::{default_app_config, DEFAULT_APP_PK_PATH, DEFAULT_APP_VK_PATH},
    util::read_to_struct_toml,
};

#[derive(Parser)]
#[command(name = "keygen", about = "Generate an application proving key")]
pub struct KeygenCmd {
    #[clap(long, action, help = "Path to app config TOML file")]
    config: Option<PathBuf>,

    #[clap(
        long,
        action,
        help = "Path to output app proving key file",
        default_value = DEFAULT_APP_PK_PATH
    )]
    output: PathBuf,

    #[clap(
        long,
        action,
        help = "Path to output app verifying key file",
        default_value = DEFAULT_APP_VK_PATH
    )]
    vk_output: PathBuf,
}

impl KeygenCmd {
    pub fn run(&self) -> Result<()> {
        let app_config: AppConfig<SdkVmConfig> = if let Some(config) = self.config.as_ref() {
            read_to_struct_toml(config)?
        } else {
            default_app_config()
        };
        let app_pk = Sdk.app_keygen(app_config)?;
        write_app_vk_to_file(app_pk.get_vk(), &self.vk_output)?;
        write_app_pk_to_file(app_pk, &self.output)?;
        Ok(())
    }
}
