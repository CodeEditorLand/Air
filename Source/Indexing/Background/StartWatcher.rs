//! # StartWatcher
//!
//! ## File: Indexing/Background/StartWatcher.rs
//!
//! ## Role in Air Architecture
//!
//! Provides background task management for the File Indexer service,
//! handling file watching startup and periodic indexing tasks.
//!
//! ## Primary Responsibility
//!
//! Start and manage background file watcher and periodic indexing tasks
//! for the indexing service.
//!
//! ## Secondary Responsibilities
//!
//! - File watcher initialization and lifecycle management
//! - Periodic background re-indexing
//! - Watcher event debouncing
//! - Background task cleanup
//!
//! ## Dependencies
//!
//! **External Crates:**
//! - `notify` - File system watching
//! - `tokio` - Async runtime for background tasks
//!
//! **Internal Modules:**
//! - `crate::Result` - Error handling type
//! - `crate::AirError` - Error types
//! - `crate::ApplicationState::ApplicationState` - Application state
//! - `super::super::FileIndexer` - Main file indexer
//! - `super::WatchFile` - File watching operations
//!
//! ## Dependents
//!
//! - `Indexing::mod::FileIndexer` - Main file indexer implementation
//!
//! ## VSCode Pattern Reference
//!
//! Inspired by VSCode's background services in
//! `src/vs/workbench/services/search/common/`
//!
//! ## Security Considerations
//!
//! - Path validation before watching
//! - Watch path limits enforcement
//! - Permission checking on watch paths
//!
//! ## Performance Considerations
//!
//! - Event debouncing prevents excessive re-indexing
//! - Parallel processing of file changes
//! - Efficient background task scheduling
//!
//! ## Error Handling Strategy
//!
//! Background tasks log errors and continue running, ensuring
//! temporary failures don't stop the indexing service.
//!
//! ## Thread Safety
//!
//! Background tasks use Arc for shared state and async/await
//! for safe concurrent operations.

use std::{path::PathBuf, sync::Arc, time::Duration};

use tokio::{
	sync::{Mutex, RwLock, Semaphore},
	task::JoinHandle,
};

use crate::{
	AirError,
	ApplicationState::ApplicationState::Struct,
	Indexing::State::CreateState::FileIndex,
	Result,
	dev_log,
};

/// Maximum number of parallel watch event processors
const MAX_WATCH_PROCESSORS:usize = 5;

/// Background indexer context containing shared state
pub struct BackgroundIndexerContext {
	/// Application state reference
	pub app_state:Arc<crate::ApplicationState::ApplicationState::Struct>,

	/// File index
	pub file_index:Arc<RwLock<FileIndex>>,

	/// Corruption detected flag
	pub corruption_detected:Arc<Mutex<bool>>,

	/// File watcher (optional)
	pub file_watcher:Arc<Mutex<Option<notify::RecommendedWatcher>>>,

	/// Semaphore for limiting parallel operations
	pub indexing_semaphore:Arc<Semaphore>,

	/// Debounced event handler
	pub debounced_handler:Arc<crate::Indexing::Watch::WatchFile::DebouncedEventHandler>,
}

impl BackgroundIndexerContext {
	pub fn new(
		app_state:Arc<crate::ApplicationState::ApplicationState::Struct>,

		file_index:Arc<RwLock<FileIndex>>,
	) -> Self {
		Self {
			app_state,

			file_index,

			corruption_detected:Arc::new(Mutex::new(false)),

			file_watcher:Arc::new(Mutex::new(None)),

			indexing_semaphore:Arc::new(Semaphore::new(MAX_WATCH_PROCESSORS)),

			debounced_handler:Arc::new(crate::Indexing::Watch::WatchFile::DebouncedEventHandler::new()),
		}
	}
}

/// Start file watcher for incremental indexing
///
/// Monitors file system changes and updates index in real-time.
/// Enables:
/// - Real-time search updates
/// - Automatic reindexing of changed files
/// - Removal of deleted files from index
pub async fn StartFileWatcher(context:&BackgroundIndexerContext, paths:Vec<PathBuf>) -> Result<()> {
	use notify::Watcher;

	let index = context.file_index.clone();

	let corruption_flag = context.corruption_detected.clone();

	let config = context.app_state.Configuration.Indexing.clone();

	let debounced_handler = context.debounced_handler.clone();

	// Create and start the watcher
	let mut watcher:notify::RecommendedWatcher = Watcher::new(
		move |res:std::result::Result<notify::Event, notify::Error>| {
			if let Ok(event) = res {
				// Check corruption flag before processing events
				if *corruption_flag.blocking_lock() {
					dev_log!(
						"indexing",
						"warn: [StartWatcher] Skipping file event - index marked as corrupted"
					);

					return;
				}

				let index = index.clone();

				// Variables cloned for use in async task
				let _index = index.clone();

				let debounced_handler = debounced_handler.clone();

				let _config_clone = config.clone();

				tokio::spawn(async move {
					// Convert event to change type and add to debounced handler
					if let Some(change_type) = crate::Indexing::Watch::WatchFile::EventKindToChangeType(event.kind) {
						for path in &event.paths {
							if crate::Indexing::Watch::WatchFile::ShouldWatchPath(
								path,
								&crate::Indexing::Watch::WatchFile::GetDefaultIgnoredPatterns(),
							) {
								debounced_handler.AddChange(path.clone(), change_type).await;
							}
						}
					}
				});
			}
		},
		notify::Config::default(),
	)
	.map_err(|e| AirError::Internal(format!("Failed to create file watcher: {}", e)))?;

	// Watch all specified paths
	for path in &paths {
		if path.exists() {
			match crate::Indexing::Watch::WatchFile::ValidateWatchPath(path) {
				Ok(()) => {
					watcher
						.watch(path, notify::RecursiveMode::Recursive)
						.map_err(|e| AirError::FileSystem(format!("Failed to watch path {}: {}", path.display(), e)))?;

					dev_log!("indexing", "[StartWatcher] Watching path: {}", path.display());
				},

				Err(e) => {
					dev_log!(
						"indexing",
						"warn: [StartWatcher] Skipping invalid watch path {}: {}",
						path.display(),
						e
					);
				},
			}
		}
	}

	*context.file_watcher.lock().await = Some(watcher);

	dev_log!(
		"indexing",
		"[StartWatcher] File watcher started successfully for {} paths",
		paths.len()
	);

	Ok(())
}

/// Start the debounce processor task
pub fn StartDebounceProcessor(context:Arc<BackgroundIndexerContext>) -> JoinHandle<()> {
	tokio::spawn(async move {
		dev_log!("indexing", "[StartWatcher] Debounce processor started");

		let interval = Duration::from_millis(100); // Process every 100ms

		// Debounce age cutoff
		let debounce_cutoff = Duration::from_millis(500);

		loop {
			tokio::time::sleep(interval).await;

			{
				// Check corruption flag
				if *context.corruption_detected.lock().await {
					dev_log!("indexing", "warn: [StartWatcher] Index corrupted, pausing debounce processing");

					tokio::time::sleep(Duration::from_secs(5)).await;

					continue;
				}

				// Process pending changes
				let config = context.app_state.Configuration.Indexing.clone();

				match context
					.debounced_handler
					.ProcessPendingChanges(debounce_cutoff, &context.file_index, &config)
					.await
				{
					Ok(changes) => {
						if !changes.is_empty() {
							dev_log!("indexing", "[StartWatcher] Processed {} debounced changes", changes.len());
						}
					},
					Err(e) => {
						dev_log!("indexing", "error: [StartWatcher] Failed to process pending changes: {}", e);
					},
				}
			}
		}
	})
}

/// Start background tasks for periodic indexing
pub async fn StartBackgroundTasks(context:Arc<BackgroundIndexerContext>) -> Result<tokio::task::JoinHandle<()>> {
	let config = &context.app_state.Configuration.Indexing;

	if !config.Enabled {
		dev_log!("indexing", "[StartWatcher] Background indexing disabled in configuration");

		return Err(AirError::Configuration("Background indexing is disabled".to_string()));
	}

	let handle = tokio::spawn(BackgroundTask(context));

	dev_log!("indexing", "[StartWatcher] Background tasks started");

	Ok(handle)
}

/// Stop background tasks
pub async fn StopBackgroundTasks(_context:&BackgroundIndexerContext) {
	dev_log!("indexing", "[StartWatcher] Stopping background tasks"); // Tasks are cancelled when the task handle is dropped
}

/// Stop file watcher
pub async fn StopFileWatcher(context:&BackgroundIndexerContext) {
	if let Some(watcher) = context.file_watcher.lock().await.take() {
		drop(watcher);

		dev_log!("indexing", "[StartWatcher] File watcher stopped");
	}
}

/// Background task for periodic indexing
async fn BackgroundTask(context:Arc<BackgroundIndexerContext>) {
	let config = context.app_state.Configuration.Indexing.clone();

	let interval = Duration::from_secs(config.UpdateIntervalMinutes as u64 * 60);

	let mut interval = tokio::time::interval(interval);

	dev_log!(
		"indexing",
		"[StartWatcher] Background indexing configured for {} minute intervals",
		config.UpdateIntervalMinutes
	);

	loop {
		interval.tick().await;

		{
			// Check corruption flag
			if *context.corruption_detected.lock().await {
				dev_log!("indexing", "warn: [StartWatcher] Index corrupted, skipping background update");

				continue;
			}

			dev_log!("indexing", "[StartWatcher] Running periodic background index...");

			// Re-index configured directories
			let directories = config.IndexDirectory.clone();

			if let Err(e) = crate::Indexing::Scan::ScanDirectory::ScanDirectory(&directories, vec![], &config, 10).await
			{
				dev_log!("indexing", "error: [StartWatcher] Background indexing failed: {}", e);
			}
		}
	}
}

/// Get watcher status
pub async fn GetWatcherStatus(context:&BackgroundIndexerContext) -> WatcherStatus {
	let is_running = context.file_watcher.lock().await.is_some();

	let pending_count = context.debounced_handler.PendingCount().await;

	let is_corrupted = *context.corruption_detected.lock().await;

	WatcherStatus { is_running, pending_count, is_corrupted }
}

/// Watcher status information
#[derive(Debug, Clone)]
pub struct WatcherStatus {
	pub is_running:bool,

	pub pending_count:usize,

	pub is_corrupted:bool,
}

/// Start all background components (watcher and tasks)
pub async fn StartAll(
	context:Arc<BackgroundIndexerContext>,

	watch_paths:Vec<PathBuf>,
) -> Result<(Option<JoinHandle<()>>, Option<JoinHandle<()>>)> {
	let watcher_handle = if config_watch_enabled(&context) {
		match StartFileWatcher(&context, watch_paths).await {
			Ok(()) => {
				// Start debounce processor
				Some(StartDebounceProcessor(context.clone()))
			},

			Err(e) => {
				dev_log!("indexing", "error: [StartWatcher] Failed to start file watcher: {}", e);

				None
			},
		}
	} else {
		None
	};

	let background_handle = match StartBackgroundTasks(context.clone()).await {
		Ok(handle) => Some(handle),

		Err(e) => {
			dev_log!("indexing", "warn: [StartWatcher] Failed to start background tasks: {}", e);

			None
		},
	};

	Ok((watcher_handle, background_handle))
}

/// Stop all background components
pub async fn StopAll(context:&BackgroundIndexerContext) {
	StopBackgroundTasks(context).await;

	StopFileWatcher(context).await;
}

/// Check if watching is enabled in configuration
fn config_watch_enabled(context:&BackgroundIndexerContext) -> bool { context.app_state.Configuration.Indexing.Enabled }
