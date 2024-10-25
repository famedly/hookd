//! Module that provides methods for handling file I/O related to running hooks
use std::{collections::HashMap, path::PathBuf};

use anyhow::Context;
use chrono::Utc;
use directories_next::ProjectDirs;
use serde_json::to_string_pretty as json_pretty_string;
use substring::Substring;
use tokio::{
	fs::{read_to_string, write, File},
	io::{copy, AsyncRead},
};
use uuid::Uuid;

use crate::{
	config::Hook,
	error::ApiError,
	model::{Info, Request},
};

/// Reads the stdout or stderr stream of a hook instance
pub async fn read_log(stream: &str, id: &Uuid, dirs: &ProjectDirs) -> Result<String, ApiError> {
	let mut log_path = get_log_dir(get_hook_data_dir(dirs, id));
	ensure_file_exists(log_path.clone(), "No hook with the matching ID was found")?;
	log_path.push(format!("{}.txt", stream));
	ensure_file_exists(
		log_path.clone(),
		"Hook with the matching ID exists, but log doesn't exist",
	)?;
	let stream = read_to_string(log_path)
		.await
		.context(format!("Couldn't read {} for instance {}", stream, id))?;
	Ok(stream)
}

/// Reads the current hook status
pub async fn read_status(id: &Uuid, dirs: &ProjectDirs) -> Result<Info, ApiError> {
	let info_path = get_info_file(get_hook_data_dir(dirs, id));
	ensure_file_exists(info_path.clone(), "No hook with the matching ID was found")?;
	let info_string = read_to_string(info_path).await.context("Couldn't read hook info")?;
	let info: Info = serde_json::from_str(&info_string)
		.context(format!("Couldn't parse json info for instance {}", id))?;
	Ok(info)
}

/// Ensures that a file exists. If it doesn't exist, this function returns an
/// `ApiError::NotFound`
pub fn ensure_file_exists(path: PathBuf, error: &'static str) -> Result<(), ApiError> {
	if path.exists() {
		Ok(())
	} else {
		Err(ApiError::NotFound(error))
	}
}

/// Function for returning the info file path and the log directory of a given
/// hook
pub fn get_hook_data_dir(dirs: &ProjectDirs, id: &Uuid) -> PathBuf {
	let mut data_dir = dirs.data_dir().to_path_buf();
	let id_string = id.hyphenated().to_string();
	for i in 0..4 {
		data_dir.push(id_string.substring(2 * i, 2 * i + 2));
	}
	data_dir.push(id_string.substring(9, id_string.len()));
	data_dir
}

pub fn get_aux_dir<P>(data_dir: P) -> PathBuf
where
	P: AsRef<std::path::Path>,
{
	let mut aux_dir = data_dir.as_ref().to_path_buf();
	aux_dir.push("aux");
	aux_dir
}

pub fn get_log_dir<P>(data_dir: P) -> PathBuf
where
	P: AsRef<std::path::Path>,
{
	let mut log_dir = data_dir.as_ref().to_path_buf();
	log_dir.push("log");
	log_dir
}

pub fn get_info_file<P>(data_dir: P) -> PathBuf
where
	P: AsRef<std::path::Path>,
{
	let mut info_file = data_dir.as_ref().to_path_buf();
	info_file.push("info.json");
	info_file
}

/// Helper function that takes the output stream of a hook instance and writes
/// it to the respective log file
pub async fn write_stream_to_file<T>(mut stream: T, path: PathBuf) -> Result<(), ApiError>
where
	T: AsyncRead + Send + Unpin,
{
	let mut file = File::create(path.clone())
		.await
		.context(format!("Couldn't create log file {}", path.to_string_lossy()))?;
	copy(&mut stream, &mut file)
		.await
		.context(format!("Couldn't write output to log file {}", path.to_string_lossy()))?;
	Ok(())
}

/// Helper function that writes the hook info after the hook has been spawned
pub async fn write_initial_hook_info(
	hook: &Hook,
	vars: &HashMap<String, String>,
	request: Request,
	file: PathBuf,
) -> Result<(), ApiError> {
	let started = Utc::now();
	let info = Info {
		request,
		config: hook.clone(),
		vars: Some(vars.clone()),
		running: true,
		success: None,
		started,
		finished: None,
	};
	let info = json_pretty_string(&info).context("Couldn't serialize hook info into string")?;
	write(file, info).await.context("Couldn't write hook info file")?;
	Ok(())
}
