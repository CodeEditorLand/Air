//! # Scan Module
//!
//! ## File: Indexing/Scan/mod.rs
//!
//! ## Role in Air Architecture
//!
//! Provides directory and file scanning functionality for the File Indexer
//! service.
//!
//! ## Modules
//!
//! - `ScanDirectory` - Recursive directory traversal scanning
//! - `ScanFile` - Individual file reading and categorization

pub mod ScanDirectory;
pub mod ScanFile;
