//! Hookd

use anyhow::Context;
use directories_next::ProjectDirs;
use hookd::{config, logging};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	let dirs = ProjectDirs::from("com", "Famedly GmbH", "hookd")
		.context("Couln't find project directory, is $HOME set?")?;
	let config = config::load(&dirs).await?;
	logging::setup(config.log_level);

	hookd::run(dirs, config).await?;
	Ok(())
}
