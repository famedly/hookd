//! Module containing struct definitions for responses.
use std::collections::HashMap;

use axum::{async_trait, extract::FromRequestParts};
use chrono::{DateTime, Utc};
use http::request::Parts;
use serde::{Deserialize, Serialize};

use crate::config::Hook;

/// Configuration for running a hook that's passed when creating a hook instance
#[derive(Deserialize, Clone, Debug)]
pub struct CreateConfig {
	/// Map of environment variables passed to the hook when executed. These
	/// will be filtered down to the allowed set configured for the hook before
	/// execution begins.
	pub vars: HashMap<String, String>,
}

impl CreateConfig {
	/// Filters the map of variables down to the allowed set of variables for
	/// this hook
	pub fn filter(&mut self, allowed_keys: &[String]) {
		self.vars.retain(|key, _val| allowed_keys.contains(key));
	}
}

/// Info about a hook instance
#[derive(Serialize, Deserialize, Debug)]
pub struct Info {
	/// Request parameters of the http request that started this hook instance
	pub request: Request,
	/// Configuration for this hook as provided in the config
	pub config: Hook,
	/// Whether the hook instance is still running
	pub running: bool,
	/// Time when the hook instance was started, UTC
	pub started: DateTime<Utc>,
	/// Time when the hook instance has finished, UTC
	#[serde(skip_serializing_if = "Option::is_none")]
	pub finished: Option<DateTime<Utc>>,
	/// Whether the hook was successful
	#[serde(skip_serializing_if = "Option::is_none")]
	pub success: Option<bool>,
}

/// Request parameters of a hook instance spawning request
#[derive(Serialize, Deserialize, Debug)]
pub struct Request {
	/// URI used for the request
	pub uri: String,
	/// Method used for the request
	pub method: String,
	/// HTTP version used for the request
	pub version: String,
	/// HTTP headers passed in the request
	pub headers: HashMap<String, String>,
}

#[async_trait]
impl<S> FromRequestParts<S> for Request {
	type Rejection = ();

	async fn from_request_parts(req: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
		let mut headers = HashMap::new();
		for (key, val) in req.headers.iter() {
			if let (key, Ok(val)) = (key.to_string(), val.to_str()) {
				headers.insert(key, val.to_owned());
			}
		}

		Ok(Self {
			uri: req.uri.to_string(),
			method: req.method.to_string(),
			version: format!("{:?}", req.version),
			headers,
		})
	}
}
