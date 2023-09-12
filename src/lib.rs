//! A simple webhook daemon that supports multiple hooks, passing env vars and
//! reading stdout/stderr.

pub mod api;
pub mod command;
pub mod config;
pub mod error;
pub mod file;
pub mod logging;
pub mod model;

use anyhow::Result;
use api::{health_check, hook_status, hook_stderr, hook_stdout, start_hook};
use axum::{
	handler::{get, post},
	AddExtensionLayer, Router,
};
use config::Config;
use directories_next::ProjectDirs;
pub use error::ApiError;

/// Main function that sets up logging and starts the API server.
pub async fn run(dirs: ProjectDirs, config: Config) -> Result<()> {
	axum::Server::bind(&config.address)
		.serve(
			Router::new()
				.route("/health", get(health_check))
				.route("/hook/:name", post(start_hook))
				.route("/status/:id", get(hook_status))
				.route("/status/:id/stdout", get(hook_stdout))
				.route("/status/:id/stderr", get(hook_stderr))
				.layer(AddExtensionLayer::new(dirs))
				.layer(AddExtensionLayer::new(config))
				.into_make_service(),
		)
		.await?;

	Ok(())
}
