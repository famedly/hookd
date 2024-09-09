//! Module with functionality for spawning hooks and handling their IO
use std::{io::SeekFrom, path::PathBuf, process::Stdio, time::Duration};

use anyhow::Context;
use chrono::Utc;
use directories_next::ProjectDirs;
use tokio::{
	fs::{create_dir_all, OpenOptions},
	io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt},
	process::{Child, Command},
	time::timeout as tokio_timeout,
};
use uuid::Uuid;

use crate::{
	config::Config,
	error::ApiError,
	file::{
		get_aux_dir, get_hook_data_dir, get_info_file, get_log_dir, write_initial_hook_info,
		write_stream_to_file,
	},
	model::{CreateConfig, Info, Request},
};

/// Runs a hook with the given configuration
pub async fn run_hook(
	config: &Config,
	req: Request,
	dirs: &ProjectDirs,
	mut create_config: CreateConfig,
	name: String,
) -> Result<Uuid, ApiError> {
	let id = Uuid::new_v4();
	let data_dir = get_hook_data_dir(dirs, &id);
	let info_path = get_info_file(&data_dir);
	let log_path = get_log_dir(&data_dir);
	let aux_path = get_aux_dir(&data_dir);
	create_dir_all(&log_path).await.context("Couldn't create hook directories")?;
	let static_config =
		config.hooks.get(&name).ok_or(ApiError::NotFound("No hook with this name configured"))?;
	create_config.filter(&static_config.allowed_keys);
	write_initial_hook_info(static_config, &create_config.vars, req, info_path.clone())
		.await
		.context("Couldn't write hook info")?;
	let mut command = Command::new(&static_config.command);
	command.stdout(Stdio::piped());
	command.stderr(Stdio::piped());
	command.current_dir(&static_config.work_dir);
	for (key, val) in &create_config.vars {
		command.env(key, val);
	}
	command.env("HOOKD_JOB_AUX_DIR", aux_path);
	let child = command.spawn().context("Couldn't spawn hook command")?;
	handle_instance(child, log_path, info_path, id, static_config.timeout)?;
	Ok(id)
}

/// Handles the newly hook instance
///
/// This includes capturing the output streams and writing them to files, as
/// well as waiting for the child process to exit and writing whether it
/// succeeded and when it finished to the info file
pub fn handle_instance(
	mut instance: Child,
	log_path: PathBuf,
	info_path: PathBuf,
	id: Uuid,
	timeout: Duration,
) -> Result<(), ApiError> {
	let mut stdout_path = log_path.clone();
	stdout_path.push("stdout.txt");
	let mut stderr_path = log_path;
	stderr_path.push("stderr.txt");
	let stdout =
		instance.stdout.take().context(format!("Couldn't take stdout of instance {}", id))?;
	let stderr =
		instance.stderr.take().context(format!("Couldn't take stderr of instance {}", id))?;
	tokio::spawn(write_stream_to_file(stdout, stdout_path));
	tokio::spawn(write_stream_to_file(stderr, stderr_path));
	tokio::spawn(update_hook_info_upon_completion(instance, info_path, id, timeout));
	Ok(())
}

/// Updates the info file for a hook after the instance completed
pub async fn update_hook_info_upon_completion(
	mut instance: Child,
	hook_info_path: PathBuf,
	id: Uuid,
	timeout: Duration,
) -> Result<(), ApiError> {
	let child_future = instance.wait();
	let timeout_future = tokio_timeout(timeout, child_future);
	let status = match timeout_future.await {
		Ok(status) => status
			.context(format!("Couldn't wait for child process to exit for instance {}", id))?,
		Err(_) => {
			instance
				.kill()
				.await
				.context(format!("Reached timeout but couldn't kill child for instance {}", id))?;
			instance.wait().await.context(format!(
				"Couldn't wait for killed child process to exit for instance {}",
				id
			))?
		}
	};
	let now = Utc::now();

	let mut info_file = OpenOptions::new()
		.read(true)
		.write(true)
		.create(false)
		.open(hook_info_path)
		.await
		.context(format!("Couldn't open info file for instance {}", id))?;
	let mut info = String::new();
	info_file
		.read_to_string(&mut info)
		.await
		.context(format!("Couldn't read info file for instance {}", id))?;
	let mut info: Info = serde_json::from_str(&info)
		.context(format!("Couldn't parse json info for instance {}", id))?;
	info.running = false;
	info.success = Some(status.success());
	info.finished = Some(now);
	let info = serde_json::to_string_pretty(&info)
		.context(format!("Couldn't serialize json info for instance {}", id))?;
	info_file
		.seek(SeekFrom::Start(0))
		.await
		.context(format!("Couldn't seek to start of info file for instance {}", id))?;
	info_file
		.write_all(&info.bytes().collect::<Vec<u8>>())
		.await
		.context(format!("Couldn't write info file for instance {}", id))?;

	Ok(())
}
