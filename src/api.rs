//! module with wrapper functions for actix handlers

use actix_web::{web, HttpRequest, HttpResponse};
use directories_next::ProjectDirs;
use uuid::Uuid;

use crate::{
    command::run_hook,
    config::Config,
    file::{read_log, read_status},
    model::CreateConfig,
};

/// Tries to read the hook status 
pub async fn hook_status(id: web::Path<Uuid>, dirs: web::Data<ProjectDirs>) -> HttpResponse {
    match read_status(&id, &dirs).await {
        Ok(info) => HttpResponse::Ok().json(info),
        Err(e) => e.into(),
    }
}

/// Tries to read the hook stdout 
pub async fn hook_stdout(id: web::Path<Uuid>, dirs: web::Data<ProjectDirs>) -> HttpResponse {
    match read_log("stdout", &id, &dirs).await {
        Ok(stdout) => HttpResponse::Ok().content_type("text/plain").body(stdout),
        Err(e) => e.into(),
    }
}

/// Tries to read the hook stderr 
pub async fn hook_stderr(id: web::Path<Uuid>, dirs: web::Data<ProjectDirs>) -> HttpResponse {
    match read_log("stderr", &id, &dirs).await {
        Ok(stderr) => HttpResponse::Ok().content_type("text/plain").body(stderr),
        Err(e) => e.into(),
    }
}

/// Starts a hook and returns it's ID
pub async fn start_hook(
    create_config: web::Json<CreateConfig>,
    path: web::Path<String>,
    config: web::Data<Config>,
    dirs: web::Data<ProjectDirs>,
    req: HttpRequest,
) -> HttpResponse {
    let name = path.into_inner();
    let create_config = create_config.0.clone();
    match run_hook(&config, req, &dirs, create_config, name).await {
        Ok(id) => HttpResponse::Ok().body(id.to_hyphenated().to_string()),
        Err(e) => e.into(),
    }
}
