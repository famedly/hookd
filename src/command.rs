//! Module with fuctionality for spawning hooks and handling their IO
use std::{io::SeekFrom, path::PathBuf, process::Stdio};

use actix_web::HttpRequest;
use anyhow::Context;
use chrono::Utc;
use directories_next::ProjectDirs;
use tokio::{
    fs::OpenOptions,
    io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt},
    process::{Child, Command},
};
use uuid::Uuid;

use crate::{
    config::Config,
    error::ApiError,
    file::{get_hook_files, write_initial_hook_info, write_stream_to_file},
    model::{CreateConfig, Info},
};

/// Runs a hook with the given configuration
pub async fn run_hook(
    config: &Config,
    req: HttpRequest,
    dirs: &ProjectDirs,
    mut create_config: CreateConfig,
    name: String,
) -> Result<Uuid, ApiError> {
    let id = Uuid::new_v4();
    let (info_path, log_path) = get_hook_files(&dirs, &id, true)
        .await
        .context("Couldn't get hook directory")?;
    let static_config = config
        .hooks
        .get(&name)
        .ok_or(ApiError::NotFound("No hook with this name configured"))?;
    create_config.filter(&static_config.allowed_keys);
    write_initial_hook_info(&static_config, req, info_path.clone())
        .await
        .context("Couldn't write hook info")?;
    let mut command = Command::new(&static_config.command);
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());
    command.current_dir(&static_config.work_dir);
    for (key, val) in &create_config.vars {
        command.env(key, val);
    }
    let child = command.spawn().context("Couldn't spawn hook command")?;
    handle_instance(child, log_path, info_path, id.clone()).await?;
    Ok(id)
}

/// Handles the newly hook instance
///
/// This includes capturing the output streams and writing them to files, as well as waiting for
/// the child process to exit and writing whether it succeeded and when it finished to the info file
pub async fn handle_instance(
    mut instance: Child,
    log_path: PathBuf,
    info_path: PathBuf,
    id: Uuid,
) -> Result<(), ApiError> {
    let mut stdout_path = log_path.clone();
    stdout_path.push("stdout.txt");
    let mut stderr_path = log_path.clone();
    stderr_path.push("stderr.txt");
    let stdout = instance
        .stdout
        .take()
        .context(format!("Couldn't take stdout of instance {}", id))?;
    let stderr = instance
        .stderr
        .take()
        .context(format!("Couldn't take stderr of instance {}", id))?;
    tokio::spawn(write_stream_to_file(stdout, stdout_path));
    tokio::spawn(write_stream_to_file(stderr, stderr_path));
    tokio::spawn(update_hook_info_upon_completion(
        instance,
        info_path,
        id.clone(),
    ));
    Ok(())
}

/// Updates the info file for a hook after the instance completed
pub async fn update_hook_info_upon_completion(
    mut instance: Child,
    hook_info_path: PathBuf,
    id: Uuid,
) -> Result<(), ApiError> {
    let status = instance.wait().await.context(format!(
        "Couldn't wait for child process to exist for instance {}",
        id
    ))?;
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
    info_file.seek(SeekFrom::Start(0)).await.context(format!(
        "Couldn't seek to start of info file for instance {}",
        id
    ))?;
    info_file
        .write_all(&info.bytes().collect::<Vec<u8>>())
        .await
        .context(format!("Couldn't write info file for instance {}", id))?;

    Ok(())
}
