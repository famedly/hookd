#![warn(missing_docs)]

//! A simple webhook daemon that supports multiple hooks, passing env vars and reading
//! stdout/stderr.

use actix_web::{web, App, HttpServer};
use anyhow::{Context, Result};
use api::{hook_status, hook_stderr, hook_stdout, start_hook};
use directories_next::ProjectDirs;

pub use error::ApiError;

pub mod api;
pub mod command;
pub mod config;
mod error;
pub mod file;
pub mod logging;
pub mod model;

/// Main function that sets up logging and starts the API server.
#[actix_web::main]
async fn main() -> Result<()> {
    let dirs = ProjectDirs::from("com", "Famedly GmbH", "hookd")
        .context("Couln't find project directory, is $HOME set?")?;
    let dirs_data = web::Data::new(dirs.clone());
    let config = config::load(&dirs).await?;
    let config_data = web::Data::new(config.clone());
    logging::setup(config.log_level);
    HttpServer::new(move || {
        App::new()
            .app_data(config_data.clone())
            .app_data(dirs_data.clone())
            .route("/hook/{name}", web::post().to(start_hook))
            .route("/status/{id}", web::get().to(hook_status))
            .route("/status/{id}/stdout", web::get().to(hook_stdout))
            .route("/status/{id}/stderr", web::get().to(hook_stderr))
    })
    .bind(config.address)?
    .run()
    .await?;
    Ok(())
}
