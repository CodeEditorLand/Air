//! Generated gRPC code for Air service

// Message definitions (snake_case - generated)
pub mod Air;

// gRPC service definitions (includes AirService trait)
pub mod air;

// Re-export message types with PascalCase for consistency
pub use Air::*;

// Re-export service trait from nested module
pub use air::air_service_server::AirService;
pub use air::air_service_server::AirServiceServer;
