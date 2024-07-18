use std::path::PathBuf;

use clap::Parser;
use config::{Config, ConfigError, File};
use serde::Deserialize;

#[derive(Debug, Clone, Parser)]
#[command(name = "Hellvents")]
pub struct CliConfig {
    #[clap()]
    pub config_file: PathBuf,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FileConfig {
    pub wise: WiseConfig,

    pub admin: AdminConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WiseConfig {
    pub address: String,
    pub token: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AdminConfig {
    pub allowed_ids: Vec<String>,
}

pub fn parse_config() -> Result<FileConfig, ConfigError> {
    let cli = CliConfig::parse();

    let config = Config::builder()
        .add_source(File::with_name(cli.config_file.to_str().unwrap()))
        .build()?;

    config.try_deserialize()
}
