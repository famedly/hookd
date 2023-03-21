//! module with wrapper functions for actix handlers
use axum::{extract, Json};
use directories_next::ProjectDirs;
use http::StatusCode;
use uuid::Uuid;

use crate::{
	command::run_hook,
	config::Config,
	file::{read_log, read_status},
	model::{CreateConfig, Info, Request},
};

// False positive
#[allow(clippy::unused_async)]
/// Always returns 200 OK, for health checking by proxies
pub async fn health_check() -> StatusCode {
	StatusCode::OK
}

/// Tries to read the hook status
pub async fn hook_status(
	id: extract::Path<Uuid>,
	dirs: extract::Extension<ProjectDirs>,
) -> Result<Json<Info>, StatusCode> {
	match read_status(&id, &dirs).await {
		Ok(info) => Ok(Json::from(info)),
		Err(e) => Err(e.into()),
	}
}

/// Tries to read the hook stdout
pub async fn hook_stdout(
	id: extract::Path<Uuid>,
	dirs: extract::Extension<ProjectDirs>,
) -> Result<String, StatusCode> {
	match read_log("stdout", &id, &dirs).await {
		Ok(stdout) => Ok(stdout),
		Err(e) => Err(e.into()),
	}
}

/// Tries to read the hook stderr
pub async fn hook_stderr(
	id: extract::Path<Uuid>,
	dirs: extract::Extension<ProjectDirs>,
) -> Result<String, StatusCode> {
	match read_log("stderr", &id, &dirs).await {
		Ok(stderr) => Ok(stderr),
		Err(e) => Err(e.into()),
	}
}

/// Starts a hook and returns it's ID
pub async fn start_hook(
	create_config: Json<CreateConfig>,
	path: extract::Path<String>,
	config: extract::Extension<Config>,
	dirs: extract::Extension<ProjectDirs>,
	req: Request,
) -> Result<String, StatusCode> {
	let name: String = path.0;
	let create_config = create_config.0.clone();
	match run_hook(&config, req, &dirs, create_config, name).await {
		Ok(id) => Ok(id.to_hyphenated().to_string()),
		Err(e) => Err(e.into()),
	}
}
