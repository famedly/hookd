//! Module with log setup for the daemon
use std::io::stdout;

use chrono::Utc;
use fern::Dispatch;
use log::{info, LevelFilter};

/// Sets up logging with `fern`
pub(crate) fn setup(level: LevelFilter) {
	match Dispatch::new()
		.format(|out, message, record| {
			out.finish(format_args!(
				"[{}][{}] {}",
				Utc::now().format("%Y-%m-%d %H:%M:%S"),
				record.level(),
				message
			))
		})
		.level(level)
		.chain(stdout())
		.apply()
	{
		Err(e) => {
			eprintln!("error setting up logging: {}", e);
		}
		_ => info!("logging set up properly"),
	}
}
