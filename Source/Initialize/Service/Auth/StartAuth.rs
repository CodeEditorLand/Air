//! # StartAuth
//!
//! ## File: Initialize/Service/Auth/StartAuth.rs
//!
//! ## Role in Air Architecture
//!
//! Initializes the authentication service that handles user authentication tokens,
//! cryptographic operations, and secure credential management. The auth service
//! provides the security foundation for all authenticated operations.
//!
//! ## Primary Responsibility
//!
//! Create and initialize the authentication service for token management.
//!
//! ## Secondary Responsibilities
//!
/// - Initialize cryptographic keys and keyrings
/// - Configure authentication providers
/// - Set up secure credential storage
//!
//! ## Dependencies
//!
//! **External Crates:**
//! - `std::sync::Arc` - Thread-safe reference counting
//! - `tokio::time` - Timeout handling
//!
//! **Internal Modules:**
//! - `AirLibrary::Authentication::AuthenticationService` - Auth service implementation
//! - `AirLibrary::ApplicationState` - Shared application state
//!
//! ## Dependents
//!
//! - `Initialize::Build::BuildServer` - Needs auth service for gRPC requests
//! - `Initialize::Service::Vine::StartService` - Routes auth requests
//!
//! ## VSCode Pattern Reference
//!
/// Inspired by VSCode's authentication service in
/// `src/vs/workbench/services/authentication/common/authenticationService.ts`
///
//! ## Security Considerations
//!
/// - Authentication service handles sensitive credentials
/// - Keys are stored securely in keyring
/// - Timeout prevents resource exhaustion
//! - All operations are defensive and validate inputs
//!
//! ## Performance Considerations
//!
/// - Initialization is done once at startup
/// - Cached token validation for performance
/// - Async operations don't block daemon startup
//!
//! ## Error Handling Strategy
///
/// - Returns descriptive errors for initialization failures
/// - Timeout prevents hanging on slow operations
/// - Errors are fatal to boot as auth is critical

use std::sync::Arc;

use std::time::Duration;

use crate::dev_log;

use AirLibrary::{
    ApplicationState,
    Authentication::AuthenticationService::AuthenticationService,
};

/// Start the authentication service
///
/// Initializes the authentication service with cryptographic operations,
/// token management, and secure credential storage capabilities.
///
/// # Arguments
///
/// * `app_state` - Shared application state for cross-service coordination
///
/// # Returns
///
/// Returns an `Arc<AuthenticationService>` for authentication operations.
///
/// # Errors
///
/// Returns an error if:
/// - Service initialization fails
/// - Cryptographic setup fails
/// - Initialization timeout occurs
///
/// # Timeout
///
/// A 10-second timeout is applied to prevent hanging on slow operations.
///
/// # Security Notes
///
/// - The auth service manages user credentials and tokens
/// - All cryptographic keys are stored in secure keyring
/// - Token refresh is handled automatically
///
/// # FUTURE Enhancements
/// - Add multi-provider authentication support
/// - Implement token revocation
/// - Add biometric authentication support
pub async fn StartAuth(
    app_state: Arc<crate::ApplicationState::ApplicationState::Struct>,
) -> Result<Arc<AuthenticationService>, String> {

    dev_log!("lifecycle", "[Auth] Starting authentication service...");    

    // Initialize auth service with timeout
    let auth_result = tokio::time::timeout(
        Duration::from_secs(10),

        AuthenticationService::new(app_state.clone())
    ).await;
    
    match auth_result {

        Ok(Ok(service)) => {

            dev_log!("lifecycle", "[Auth] Authentication service initialized successfully");            Ok(Arc::new(service))
        }

        Ok(Err(e)) => {

            dev_log!("lifecycle", "error: [Auth] Failed to initialize authentication service: {}", e);            Err(format!("Authentication service initialization failed: {}", e))
        }

        Err(_) => {

            dev_log!("lifecycle", "error: [Auth] Authentication service initialization timed out");            Err("Authentication service initialization timed out".to_string())
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    
    #[test]
    #[ignore] // Requires actual app state
    async fn test_start_auth() {

        // This test requires proper application state setup
        // and is ignored for automated test runs.
    }
}
