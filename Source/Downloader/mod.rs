//! # Download Manager Service
//!
//! Central download authority across the Land ecosystem: extension VSIX
//! downloads, package fetching, runtime updates, and asset management.
//! Based on VSCode's download service patterns with resilience, streaming,
//! verification, and priority queuing.

pub mod DownloadManager;

pub mod RateLimit;

pub mod Types;
