//! # File Indexing and Search Service
//!
//! ## File: Indexing/mod.rs
//!
//! ## Role in Air Architecture
//!
//! Provides comprehensive file indexing, search, and content analysis
//! capabilities for the Land ecosystem, inspired by and compatible with
//! Visual Studio Code's search service.
//!
//! ## Primary Responsibility
//!
//! Facade module for the Indexing service, exposing the public API for
//! file indexing, search, and symbol extraction operations.
//!
//! ## Secondary Responsibilities
//!
//! - Re-export public types from submodule
//! - Provide unified FileIndexer API
//! - Coordinate between indexing subsystems
//!
//! ## Dependencies
//!
//! **External Crates:**
//! - `regex` - Regular expression search patterns
//! - `serde` - Serialization for index storage
//! - `tokio` - Async runtime for all operations
//! - `notify` - File system watching
//! - `chrono` - Timestamp management
//!
//! **Internal Modules:**
//! - `crate::Result` - Error handling type
//! - `crate::AirError` - Error types
//! - `crate::ApplicationState::ApplicationState` - Application state
//! - `crate::Configuration::ConfigurationManager` - Configuration management
//!
//! ## Dependents
//!
//! - `Indexing::FileIndexer` - Main indexer implementation
//! - `Vine::Server::AirVinegRPCService` - gRPC integration
//!
//! ## VSCode Integration
//!
//! This service integrates with VSCode's search and file service architecture:
//!
//! - References: vs/workbench/services/search
//! - File Service: vs/workbench/services/files
//!
//! The indexing system supports VSCode features:
//! - **Outline View**: Symbol extraction for class/function navigation
//! - **Go to Symbol**: Cross-file symbol search and lookup
//! - **Search Integration**: File content and name search with regex support
//! - **Workspace Search**: Multi-workspace index sharing
//!
//! ## FUTURE Enhancements
//!
//! - [ ] Implement full ripgrep integration for ultra-fast text search
//! - [ ] Add project-level search with workspace awareness
//! - [ ] Implement search query caching
//! - [ ] Add fuzzy search with typos tolerance
//! - [ ] Implement search history and recent queries
//! - [ ] Add search result preview with context
//! - [ ] Implement parallel indexing for large directories

// Submodule declarations
pub mod State;

pub mod Scan;

pub mod Process;

pub mod Language;

pub mod Store;

pub mod Watch;

pub mod Background;

// Inline-extracted item files
pub mod FileIndexer;

pub mod IndexResult;
