//! # Air::Client::AirServiceProvider
//!
//! High-level wrappers around [`crate::Client::AirClient`] that hide the
//! gRPC plumbing behind ergonomic methods (`SearchFiles`, `GetMetrics`,
//! `CheckForUpdates`, `AuthenticateUser`, `DownloadFile`, …). Each method
//! generates a request id via [`GenerateRequestID::Fn`], issues the gRPC
//! call, and translates errors into [`crate::AirError`].

pub mod GenerateRequestID;
