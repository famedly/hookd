//! module with wrapper functions for actix handlers
use anyhow::Result;
use axum::{extract, Json};
use directories_next::ProjectDirs;
use headers::{ContentRange, HeaderMapExt, Range};
use http::{HeaderMap, StatusCode};
use uuid::Uuid;

use crate::{
    command::run_hook,
    config::Config,
    file::{read_log, read_status},
    model::{CreateConfig, Info, Request},
};

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

/// Implementation for [`hook_stdout`] and [`hook_stderr`]
async fn hook(
    stream: &str,
    id: extract::Path<Uuid>,
    dirs: extract::Extension<ProjectDirs>,
    range: Option<extract::TypedHeader<Range>>,
) -> Result<(StatusCode, HeaderMap, String), StatusCode> {
    let mut headers = HeaderMap::new();
    headers.typed_insert(headers::AcceptRanges::bytes());

    let requested_range = match &range {
        Some(extract::TypedHeader(range)) => {
            // we don't support multirange requests, so check if there was only one range requested
            let mut iter = range.iter();
            let range = iter.next();
            if iter.next().is_none() {
                range
            } else {
                None
            }
        }
        None => None,
    };

    let (stdout, range) = match read_log(stream, &id, &dirs, requested_range).await {
        Ok(stdout) => stdout,
        Err(e) => {
            return Err(e.into());
        }
    };

    if let Some(range) = range {
        let range = ContentRange::bytes(range, None).unwrap();
        headers.typed_insert(range);

        Ok((StatusCode::PARTIAL_CONTENT, headers, stdout))
    } else {
        Ok((StatusCode::OK, headers, stdout))
    }
}

/// Tries to read the hook stdout
pub async fn hook_stdout(
    id: extract::Path<Uuid>,
    dirs: extract::Extension<ProjectDirs>,
    range: Option<extract::TypedHeader<Range>>,
) -> Result<(StatusCode, HeaderMap, String), StatusCode> {
    hook("stdout", id, dirs, range).await
}

/// Tries to read the hook stderr
pub async fn hook_stderr(
    id: extract::Path<Uuid>,
    dirs: extract::Extension<ProjectDirs>,
    range: Option<extract::TypedHeader<Range>>,
) -> Result<(StatusCode, HeaderMap, String), StatusCode> {
    hook("stderr", id, dirs, range).await
}

/// Starts a hook and returns it's ID
pub async fn start_hook(
    create_config: extract::Json<CreateConfig>,
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
