//! Module with log setup for the daemon
use std::io::stdout;

use chrono::Utc;
use fern::Dispatch;
use log::{info, LevelFilter};

#[allow(clippy::print_stderr)]
/// Sets up logging with `fern`
pub fn setup(level: LevelFilter) {
	let dispatch = Dispatch::new()
		.format(|out, message, record| {
			out.finish(format_args!(
				"[{}][{}] {}",
				Utc::now().format("%Y-%m-%d %H:%M:%S"),
				record.level(),
				message
			));
		})
		.level(level)
		.chain(stdout());
	match dispatch.apply() {
		Err(e) => {
			eprintln!("error setting up logging: {}", e);
		}
		_ => info!("logging set up properly"),
	}
}
