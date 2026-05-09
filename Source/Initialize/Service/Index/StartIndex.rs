//! # StartIndex
//!
//! ## File: Initialize/Service/Index/StartIndex.rs
//!
//! ## Role in Air Architecture
//!
//! Initializes the file indexer that maintains a searchable index of project files
//! for code navigation features like "Go to Definition" and "Find All References."
//! The indexer enables fast file searching and symbol extraction.
//!
//! ## Primary Responsibility
//!
/// Create and initialize the file indexer for code navigation and search.
//!
//! ## Secondary Responsibilities
//!
//! - Configure indexing scan intervals
//! - Set up file watching for incremental updates
//! - Initialize symbol extraction parsers
//!
//! ## Dependencies
//!
//! **External Crates:**
//! - `std::sync::Arc` - Thread-safe reference counting
//! - `tokio::time` - Timeout handling
//!
//! **Internal Modules:**
//! - `AirLibrary::Indexing::FileIndexer` - File indexer implementation
//! - `AirLibrary::ApplicationState` - Shared application state
//!
//! ## Dependents
//!
//! - `Initialize::Build::BuildServer` - Needs indexer for gRPC requests
//! - `Initialize::Service::Vine::StartService` - Routes indexing requests
//!
//! ## VSCode Pattern Reference
//!
//! Inspired by VSCode's search service in
//! `src/vs/workbench/services/search/common/searchService.ts`
//!
//! ## Security Considerations
//!
//! - Path validation prevents directory traversal
//! - .gitignore and exclude patterns respected
//! - Sensitive files can be excluded from index
//!
//! ## Performance Considerations
//!
//! - Incremental updates only scan changed files
//! - Parallel scanning of directories
//! - Cached search results for common queries
//!
//! ## Error Handling Strategy
//!
//! - Indexer failures don't halt daemon
//! - Corrupted indexes are automatically rebuilt
//! - Temporary errors are retried automatically

use std::sync::Arc;

use std::time::Duration;

use crate::dev_log;

use AirLibrary::{
    ApplicationState,
    Indexing::FileIndexer,
};

/// Start the file indexer
///
/// Initializes the file indexer with support for background file scanning,
//! incremental updates, and fast search operations for code navigation.
///
/// # Arguments
///
/// * `app_state` - Shared application state for cross-service coordination
///
/// # Returns
///
/// Returns an `Arc<FileIndexer>` for file indexing operations.
///
/// # Errors
///
/// Returns an error if:
//! - Service initialization fails
//! - Symbol parser setup fails
//! - Initialization timeout occurs
//!
//! # Timeout
//!
/// A 10-second timeout is applied to prevent hanging on slow operations.
///
//! # Indexing Features
//!
//! - **Incremental**: File watching for real-time updates
//! - **Parallel**: Multi-threaded directory scanning
//! - **Symbol Extraction**: Classes, functions, and methods
//! - **Fast Search**: Inverted index for quick lookups
//! - **Exclusion Patterns**: Respects .gitignore and custom excludes
//!
//! # FUTURE Enhancements
//! - Add language-specific parsing optimizations
//! - Implement fuzzy search capability
//! - Add symbol relationship tracking

pub async fn StartIndex(
    app_state: Arc<ApplicationState>,
) -> Result<Arc<FileIndexer>, String> {

    dev_log!("lifecycle", "[Index] Starting file indexer...");    

    // Initialize file indexer with timeout
    let indexer_result = tokio::time::timeout(
        Duration::from_secs(10),

        FileIndexer::new(app_state.clone())
    ).await;
    
    match indexer_result {

        Ok(Ok(indexer)) => {

            dev_log!("lifecycle", "[Index] File indexer initialized successfully");            Ok(Arc::new(indexer))
        }

        Ok(Err(e)) => {

            dev_log!("lifecycle", "error: [Index] Failed to initialize file indexer: {}", e);            Err(format!("File indexer initialization failed: {}", e))
        }

        Err(_) => {

            dev_log!("lifecycle", "error: [Index] File indexer initialization timed out");            Err("File indexer initialization timed out".to_string())
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    
    #[test]
    #[ignore] // Requires actual app state
    async fn test_start_index() {

        // This test requires proper application state setup
        // and is ignored for automated test runs.
    }
}
