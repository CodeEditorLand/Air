//! # WaitForShutdownSignal
//!
//! ## File: Binary/Shutdown/WaitForShutdownSignal.rs
//!
//! ## Role in Air Architecture
//!
//! Waits for termination signals (SIGINT/SIGTERM) to initiate graceful
//! shutdown. This is the primary mechanism for cleanly stopping the Air daemon.
//!
//! ## Primary Responsibility
//!
//! Wait for OS termination signals and trigger graceful shutdown.
//!
//! ## Secondary Responsibilities
//!
//! - Handle Ctrl+C (SIGINT) signal
//! - Handle SIGTERM signal (Unix/Linux/macOS)
//! - Log shutdown signal receipt
//!
//! ## Dependencies
//!
//! **External Crates:**
//! - `tokio::signal` - Async signal handling
//!
//! ## Dependents
//!
//! - `Binary::Binary::Main` - Waits for shutdown signal at runtime
//!
//! ## VSCode Pattern Reference
//!
//! Inspired by VSCode's shutdown handling in
//! `src/vs/base/node/shutdown.ts`
//!
//! ## Security Considerations
//!
//! - Only signals from authorized sources should trigger shutdown
//! - FUTURE: Implement signal source verification
//!
//! ## Performance Considerations
//!
//! - Signal handler is lightweight
//! - No blocking operations in signal path
//!
//! ## Error Handling Strategy
//!
//! - Logs errors if signal handler fails to install
//! - Continues operation if handler installation fails
//!
//! ## Thread Safety
//!
//! - Async signal handling is safe with tokio
//! - No mutable state shared across threads

use crate::dev_log;

/// Shutdown signal handler for graceful termination
///
/// Waits for either Ctrl+C (SIGINT) or SIGTERM signals to initiate graceful
/// shutdown, and then initiates the shutdown sequence. It has no wall-clock
/// deadline: a failed handler installation ends its own arm rather than
/// starting a timer.
///
/// # Behavior
///
/// - Listens for SIGINT (Ctrl+C) on all platforms
/// - Listens for SIGTERM on Unix/Linux/macOS
/// - Logs when signal is received
/// - Returns immediately after first signal
///
/// # Platform Notes
///
/// - **Unix/Linux/macOS**: Handles both SIGINT and SIGTERM
/// - **Windows**: Only handles SIGINT (Ctrl+C)
///
/// # FUTURE Enhancements
/// - Add configurable shutdown timeout (currently infinite)
/// - Implement signal handling for SIGHUP (reload config)
/// - Add graceful timeout with pending operation completion
///
/// # Examples
///
/// ```no_run
/// # async fn Example() {
/// // In main daemon loop
/// WaitForShutdownSignal().await;
/// // Perform cleanup
/// # }
/// ```
/// Waits for a termination signal (SIGINT/Ctrl+C, or SIGTERM on Unix).
///
/// This is the entry point used by `Binary::run`, reached as
/// `crate::Binary::Shutdown::WaitForShutdownSignal::wait()` - the module path,
/// because the `WaitForShutdownSignal` facade is a module, not a type.
///
/// # Defensive behaviour
///
/// - Signal-handler installation failures are logged as warnings and never
///   panic the daemon.
/// - A closed or missing signal channel is detected when `recv()` returns
///   `None`, logged, and returned from gracefully, so a broken channel is not
///   mistaken for a received signal.
/// - `tokio::select!` gives structured cancellation: dropping this future
///   cancels both arms safely, with no panic and no leaked handler.
///
/// The wait ends when a shutdown signal arrives, the signal channel closes,
/// or a handler cannot be installed; there is no wall-clock deadline. It
/// returns `Ok(())` once that happens so the daemon shuts down gracefully,
/// preserving the behaviour of the original free function below.
pub async fn wait() -> Result<(), String> {
	dev_log!("lifecycle", "[Shutdown] Waiting for termination signal...");

	let CtrlC = async {
		match tokio::signal::ctrl_c().await {
			Ok(()) => dev_log!("lifecycle", "[Shutdown] Received Ctrl+C signal"),

			Err(Error) => dev_log!("lifecycle", "warning: [Shutdown] Failed to install Ctrl+C handler; ending the wait and initiating graceful shutdown: {}", Error),
		}
	};

	#[cfg(unix)]
	let Terminate = async {
		match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
			Ok(mut Signal) => match Signal.recv().await {
				Some(()) => dev_log!("lifecycle", "[Shutdown] Received SIGTERM signal"),

				None => dev_log!("lifecycle", "warning: [Shutdown] SIGTERM channel closed before a signal arrived; returning gracefully"),
			},

			Err(Error) => dev_log!("lifecycle", "warning: [Shutdown] Failed to install SIGTERM handler; ending the wait and initiating graceful shutdown: {}", Error),
		}
	};

	#[cfg(not(unix))]
	let Terminate = async {
		dev_log!("lifecycle", "warning: [Shutdown] SIGTERM unavailable on this platform; waiting for Ctrl+C only");
		std::future::pending::<()>().await;
	};

	tokio::select! {

		_ = CtrlC => {},

		_ = Terminate => {},
	}

	dev_log!("lifecycle", "[Shutdown] Signal received or channel closed, initiating graceful shutdown");

	Ok(())
}

/// Backwards-compatible wrapper around [`wait`].
///
/// Preserves the original free-function API. All behaviour is delegated to
/// [`wait`]; an error would be logged rather than panicking the daemon.
pub async fn WaitForShutdownSignal() {
	if let Err(Error) = wait().await {
		dev_log!("lifecycle", "warning: [Shutdown] wait() reported an error: {}", Error);
	}
}

#[cfg(test)]
mod tests {

	use super::*;

	#[test]
	#[ignore] // Requires manual signal sending
	#[tokio::test]
	async fn TestWaitForShutdownSignal() {

		// This test requires manual signal sending
		// and is ignored for automated test runs.
	}
}
