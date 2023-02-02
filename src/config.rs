//! Module for handling the configuration of the service
use std::{collections::HashMap, net::SocketAddr};

use anyhow::Result;
use directories_next::ProjectDirs;
use log::LevelFilter;
use serde::{Deserialize, Serialize};
use tokio::{fs::File, io::AsyncReadExt};

/// Service configuration
#[derive(Deserialize, Clone)]
pub struct Config {
	/// Map from hook name to hook config
	pub hooks: HashMap<String, Hook>,
	/// Address to bind to
	pub address: SocketAddr,
	/// Log level for the daemon
	pub log_level: LevelFilter,
}

/// Configuration for a specific hook
#[derive(Serialize, Deserialize, Clone)]
pub struct Hook {
	/// Command to execute in the hook
	pub command: String,
	/// Working directory for the command
	pub work_dir: String,
	/// Allowed env var keys for including in the hook
	pub allowed_keys: Vec<String>,
}

/// Tries to load the service config
pub async fn load(dirs: &ProjectDirs) -> Result<Config> {
	let mut file = dirs.config_dir().to_path_buf();
	file.push("config.yaml");
	let mut file = File::open(file).await?;
	let mut config = String::new();
	file.read_to_string(&mut config).await?;
	let config: Config = serde_yaml::from_str(&config)?;
	Ok(config)
}
