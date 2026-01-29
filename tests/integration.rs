//! # Air Integration Tests
//!
//! Comprehensive integration testing suite for Air's ecosystem integration
//! with Mountain, Wind, and Cocoon components.
//!
//! This module provides tests for:
//! - Mountain gRPC communication and protocol compatibility
//! - Wind UI component synchronization and event handling
//! - Cocoon VS Code extension hosting workflows
//! - Performance under concurrent load conditions
//! - Error recovery and ecosystem resilience

#![cfg(test)]

use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tonic::{Request, Response, Status};

use Air::{
    ApplicationState::{ApplicationState, ConnectionType},
    Authentication::AuthenticationService,
    Configuration::ConfigurationManager,
    Downloader::DownloadManager,
    Indexing::FileIndexer,
    Updates::UpdateManager,
    Vine::Server::AirVinegRPCService,
    DEFAULT_BIND_ADDRESS,
};

mod mock_services;
mod mountain_integration;
mod wind_coordination;
mod cocoon_extension;
mod performance_tests;
mod ecosystem_validation;

use mock_services::{
    MockMountainService, MockWindService, MockCocoonService,
    MockAuthenticationService, MockUpdateManager, MockDownloadManager, MockFileIndexer
};

/// Integration test utilities and helpers
pub mod utils {
    use super::*;
    
    /// Create a test Air service instance with mock dependencies
    pub async fn create_test_air_service() -> Arc<AirVinegRPCService> {
        let app_state = Arc::new(ApplicationState::new());
        let auth_service = Arc::new(MockAuthenticationService::new());
        let update_manager = Arc::new(MockUpdateManager::new());
        let download_manager = Arc::new(MockDownloadManager::new());
        let file_indexer = Arc::new(MockFileIndexer::new());
        
        Arc::new(AirVinegRPCService::new(
            app_state,
            auth_service,
            update_manager,
            download_manager,
            file_indexer,
        ))
    }
    
    /// Wait for a condition with timeout
    pub async fn wait_for_condition<F>(mut condition: F, timeout_ms: u64) -> bool 
    where 
        F: FnMut() -> bool 
    {
        let start = std::time::Instant::now();
        while start.elapsed().as_millis() < timeout_ms as u128 {
            if condition() {
                return true;
            }
            sleep(Duration::from_millis(10)).await;
        }
        false
    }
    
    /// Generate test configuration
    pub fn test_configuration() -> ConfigurationManager {
        ConfigurationManager::new("test-config.toml").unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use utils::create_test_air_service;
    
    #[tokio::test]
    async fn test_air_service_creation() {
        let service = create_test_air_service().await;
        assert!(Arc::strong_count(&service) >= 1);
    }
    
    #[tokio::test]
    async fn test_default_bind_address() {
        assert_eq!(DEFAULT_BIND_ADDRESS, "[::1]:50053");
    }
}
