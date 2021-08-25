use http::StatusCode;
use thiserror::Error;

/// Error for describing why a request failed
#[derive(Error, Debug)]
pub enum ApiError {
    /// There has been an unexpected fatal error that prevented execution of the requested action
    #[error("Couldn't handle request: {0}")]
    InternalServerError(#[from] anyhow::Error),
    /// The resource requested does not exist
    #[error("Resource not found: {0}")]
    NotFound(&'static str),
    /// The range requested can't be satisfied (e.g. range begins after end of file or not at a char boundary)
    #[error("Range not satisfiable: {0}")]
    RangeNotSatisfiable(&'static str),
    /// The requested range is semantically wrong (e.g. range start is greater than range end)
    #[error("Invalid range: {0}")]
    InvalidRange(&'static str),
}

impl From<ApiError> for StatusCode {
    fn from(e: ApiError) -> Self {
        match e {
            ApiError::InternalServerError(e) => {
                log::error!(
                    "Couldn't handle request! See: {}, caused by {}",
                    e,
                    e.root_cause()
                );
                StatusCode::INTERNAL_SERVER_ERROR
            }
            ApiError::NotFound(e) => {
                log::debug!("Resource not found! See: {}", e);
                StatusCode::NOT_FOUND
            }
            ApiError::RangeNotSatisfiable(e) => {
                log::debug!("Range not satisfiable! See: {}", e);
                StatusCode::RANGE_NOT_SATISFIABLE
            }
            ApiError::InvalidRange(e) => {
                log::debug!("Invalid range request! See: {}", e);
                StatusCode::BAD_REQUEST
            }
        }
    }
}
