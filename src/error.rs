use http::StatusCode;
use thiserror::Error;

/// Error for describing why a request failed
#[derive(Error, Debug)]
pub enum ApiError {
	/// There has been an unexpected fatal error that prevented execution of the
	/// requested action
	#[error("Couldn't handle request: {0}")]
	InternalServerError(#[from] anyhow::Error),
	/// The resource requested does not exist
	#[error("Resource not found: {0}")]
	NotFound(&'static str),
}

impl From<ApiError> for StatusCode {
	fn from(e: ApiError) -> Self {
		match e {
			ApiError::InternalServerError(e) => {
				log::error!("Couldn't handle request! See: {}, caused by {}", e, e.root_cause());
				StatusCode::INTERNAL_SERVER_ERROR
			}
			ApiError::NotFound(e) => {
				log::debug!("Resource not found! See: {}", e);
				StatusCode::NOT_FOUND
			}
		}
	}
}
