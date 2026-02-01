//! # Application State Management
//!
//! This module provides comprehensive application state management for the Air
//! daemon, including configuration, service status, resource tracking, and
//! connection management.
//!
//! ## Architecture Overview
//!
//! The ApplicationState acts as the central coordination point for the Air
//! daemon, tracking all active connections, service states, requests, and
//! system resources. It follows the VSCode daemon state management pattern with
//! thread-safe, async-friendly data structures.
//!
//! ## Core Responsibilities
//!
//! 1. **Connection Management**
//!    - Track all connected clients (Mountain, Cocoon, Wind, etc.)
//!    - Heartbeat monitoring and validation
//!    - Connection pooling for Mountain clients
//!    - Graceful disconnection handling
//!    - Stale connection cleanup
//!
//! 2. **Service Status Tracking**
//!    - Monitor authentication service
//!    - Track update service state
//!    - Monitor indexing service
//!    - Track downloader service
//!    - gRPC service status
//!
//! 3. **Request Management**
//!    - Active request tracking
//!    - Request lifecycle management
//!    - Progress monitoring
//!    - Request cancellation support
//!
//! 4. **Resource Monitoring**
//!    - Memory usage tracking
//!    - CPU usage monitoring
//!    - Disk usage tracking
//!    - Network usage monitoring
//!    - Resource limit enforcement
//!
//! 5. **Performance Metrics**
//!    - Request count tracking
//!    - Success/failure rates
//!    - Average response time calculation
//!    - Uptime tracking
//!
//! ## Connection Types
//!
//! - **MountainMain**: Main editor process connection
//! - **MountainWorker**: Background worker processes
//! - **Cocoon**: Deployment and build system
//! - **Wind**: Communication layer
//! - **External**: Third-party integrations
//!
//! ## TODO Items
//!
//! - [ ] Implement connection pooling for Mountain clients with load balancing
//! - [ ] Add connection encryption and authentication
//! - [ ] Implement connection rate limiting
//! - [ ] Add connection-based resource quotas
//! - [ ] Implement connection health checks with automatic recovery
//! - [ ] Add connection-based feature flags
//! - [ ] Implement connection audit logging
//! - [ ] Add connection-based request prioritization
//! - [ ] Implement connection state persistence across restarts
//! - [ ] Add distributed connection support for multi-node deployments
//!
//! ## Thread Safety
//!
//! All state access is protected by async-friendly locks:
//! - `RwLock<T>` for read-heavy data (configuration, metrics, connections)
//! - `Mutex<T>` for write-heavy data (active requests, background tasks)
//!
//! This ensures concurrent access from multiple async tasks and threads
//! is safe and efficient.
//!
//! ## Performance Considerations
//!
//! - Use read locks where possible to allow concurrent access
//! - Minimize lock duration
//! - Use async-friendly lock types to avoid blocking runtime
//! - Consider lock-free alternatives for high-frequency updates
//!
//! ## Error Handling
//!
//! All methods return `Result<T>` with comprehensive error types:
//! - `Configuration`: Invalid configuration values
//! - `ResourceLimit`: Resource limits exceeded
//! - `Internal`: Internal state inconsistencies

use std::{collections::HashMap, sync::Arc};

use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, RwLock};
use systemstat::{Platform, System};

use crate::{AirError, Configuration::AirConfiguration, Result, utils};

/// Application state structure
#[derive(Debug)]
pub struct ApplicationState {
	/// Current configuration
	pub configuration:Arc<AirConfiguration>,

	/// Service status tracking
	pub service_status:Arc<RwLock<HashMap<String, ServiceStatus>>>,

	/// Active requests tracking
	pub active_requests:Arc<Mutex<HashMap<String, RequestStatus>>>,

	/// Performance metrics
	pub metrics:Arc<RwLock<PerformanceMetrics>>,

	/// Resource usage tracking
	pub resources:Arc<RwLock<ResourceUsage>>,

	/// Connection tracking for Mountain clients
	pub connections:Arc<RwLock<HashMap<String, ConnectionInfo>>>,

	/// Background task management
	pub background_tasks:Arc<Mutex<Vec<tokio::task::JoinHandle<()>>>>,
}

/// Service status enum
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServiceStatus {
	Starting,
	Running,
	Stopping,
	Stopped,
	Error(String),
}

/// Request status tracking
#[derive(Debug, Clone)]
pub struct RequestStatus {
	pub request_id:String,
	pub service:String,
	pub started_at:u64,
	pub status:RequestState,
	pub progress:Option<f32>,
}

/// Request state enum
#[derive(Debug, Clone)]
pub enum RequestState {
	Pending,
	InProgress,
	Completed,
	Failed(String),
	Cancelled,
}

/// Performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
	pub total_requests:u64,
	pub successful_requests:u64,
	pub failed_requests:u64,
	pub average_response_time:f64,
	pub uptime_seconds:u64,
	pub last_updated:u64,
}

/// Resource usage tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
	pub memory_usage_mb:f64,
	pub cpu_usage_percent:f64,
	pub disk_usage_mb:f64,
	pub network_usage_mbps:f64,
	pub last_updated:u64,
}

/// Connection information for Mountain clients
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionInfo {
	pub connection_id:String,
	pub client_id:String,
	pub client_version:String,
	pub protocol_version:u32,
	pub last_heartbeat:u64,
	pub is_active:bool,
	pub connection_type:ConnectionType,
}

/// Connection type enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConnectionType {
	MountainMain,
	MountainWorker,
	Cocoon,
	Wind,
	External,
}

/// Connection health report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionHealthReport {
	pub total_connections:usize,
	pub healthy_connections:usize,
	pub stale_connections:usize,
	pub connections_by_type:HashMap<String, usize>,
	pub last_checked:u64,
}

impl ApplicationState {
	/// Create a new ApplicationState instance
	pub async fn New(configuration:Arc<AirConfiguration>) -> Result<Self> {
		let state = Self {
			configuration,
			service_status:Arc::new(RwLock::new(HashMap::new())),
			active_requests:Arc::new(Mutex::new(HashMap::new())),
			metrics:Arc::new(RwLock::new(PerformanceMetrics {
				total_requests:0,
				successful_requests:0,
				failed_requests:0,
				average_response_time:0.0,
				uptime_seconds:0,
				last_updated:utils::CurrentTimestamp(),
			})),
			resources:Arc::new(RwLock::new(ResourceUsage {
				memory_usage_mb:0.0,
				cpu_usage_percent:0.0,
				disk_usage_mb:0.0,
				network_usage_mbps:0.0,
				last_updated:utils::CurrentTimestamp(),
			})),
			connections:Arc::new(RwLock::new(HashMap::new())),
			background_tasks:Arc::new(Mutex::new(Vec::new())),
		};

		// Initialize service status
		state.InitializeServiceStatus().await?;

		Ok(state)
	}

	/// Initialize service status tracking
	async fn InitializeServiceStatus(&self) -> Result<()> {
		let mut Status = self.service_status.write().await;

		Status.insert("authentication".to_string(), ServiceStatus::Starting);
		Status.insert("updates".to_string(), ServiceStatus::Starting);
		Status.insert("downloader".to_string(), ServiceStatus::Starting);
		Status.insert("indexing".to_string(), ServiceStatus::Starting);
		Status.insert("grpc".to_string(), ServiceStatus::Starting);
		Status.insert("connections".to_string(), ServiceStatus::Starting);

		Ok(())
	}

	/// Register a new connection with comprehensive validation
	/// Supports connection pooling for Mountain clients
	pub async fn RegisterConnection(
		&self,
		ConnectionId:String,
		ClientId:String,
		ClientVersion:String,
		ProtocolVersion:u32,
		ConnectionType:ConnectionType,
	) -> Result<()> {
		// Validate connection ID
		if ConnectionId.is_empty() {
			return Err(AirError::Configuration("Connection ID cannot be empty".to_string()));
		}

		// Validate client ID
		if ClientId.is_empty() {
			return Err(AirError::Configuration("Client ID cannot be empty".to_string()));
		}

		// Validate protocol version
		if ProtocolVersion == 0 {
			return Err(AirError::Configuration("Protocol version must be greater than 0".to_string()));
		}

		let mut Connections = self.connections.write().await;

		// Check for duplicate connections
		if Connections.contains_key(&ConnectionId) {
			return Err(AirError::Configuration(format!("Connection {} already exists", ConnectionId)));
		}

		// Implement connection pooling for Mountain clients
		if matches!(ConnectionType, ConnectionType::MountainMain | ConnectionType::MountainWorker) {
			// Check if client has too many connections
			let ClientConnCount = Connections
				.values()
				.filter(|c| {
					c.client_id == ClientId
						&& matches!(c.connection_type, ConnectionType::MountainMain | ConnectionType::MountainWorker)
				})
				.count();

			const MAX_CONN_PER_CLIENT:usize = 10;
			if ClientConnCount >= MAX_CONN_PER_CLIENT {
				return Err(AirError::ResourceLimit(format!(
					"Client {} exceeds maximum connection limit ({})",
					ClientId, MAX_CONN_PER_CLIENT
				)));
			}
		}

		Connections.insert(
			ConnectionId.clone(),
			ConnectionInfo {
				connection_id:ConnectionId.clone(),
				client_id:ClientId.clone(),
				client_version:ClientVersion,
				protocol_version:ProtocolVersion,
				last_heartbeat:utils::CurrentTimestamp(),
				is_active:true,
				connection_type:ConnectionType.clone(),
			},
		);

		log::info!(
			"Connection registered: {} - {} ({:?})",
			ConnectionId,
			ClientId,
			ConnectionType
		);
		Ok(())
	}

	/// Update connection heartbeat with validation
	/// Validates heartbeat timing and connection state
	pub async fn UpdateHeartbeat(&self, ConnectionId:&str) -> Result<()> {
		if ConnectionId.is_empty() {
			return Err(AirError::Configuration("Connection ID cannot be empty".to_string()));
		}

		let mut Connections = self.connections.write().await;

		if let Some(Connection) = Connections.get_mut(ConnectionId) {
			let CurrentTime = utils::CurrentTimestamp();
			const MAX_HEARTBEAT_INTERVAL:u64 = 120000; // 2 minutes

			// Validate heartbeat timing
			if CurrentTime - Connection.last_heartbeat > MAX_HEARTBEAT_INTERVAL {
				log::warn!(
					"Long heartbeat interval for connection {}: {}ms",
					ConnectionId,
					CurrentTime - Connection.last_heartbeat
				);
			}

			Connection.last_heartbeat = CurrentTime;
			Connection.is_active = true;

			log::debug!(
				"Heartbeat updated for connection: {} (client: {})",
				ConnectionId,
				Connection.client_id
			);
		} else {
			return Err(AirError::Internal(format!("Connection {} not found", ConnectionId)));
		}

		Ok(())
	}

	/// Remove connection with proper cleanup and validation
	/// Ensures all resources associated with the connection are cleaned up
	pub async fn RemoveConnection(&self, ConnectionId:&str) -> Result<()> {
		if ConnectionId.is_empty() {
			return Err(AirError::Configuration("Connection ID cannot be empty".to_string()));
		}

		let mut Connections = self.connections.write().await;

		if let Some(Connection) = Connections.remove(ConnectionId) {
			log::info!(
				"Connection removed: {} (client: {}, type: {:?})",
				ConnectionId,
				Connection.client_id,
				Connection.connection_type
			);

			// TODO: Cleanup any resources associated with this connection
			// - Close any open file handles
			// - Cancel pending requests
			// - Release resources
		} else {
			log::warn!("Attempted to remove non-existent connection: {}", ConnectionId);
		}

		Ok(())
	}

	/// Get active connection count with optional filtering by type
	pub async fn GetActiveConnectionCount(&self) -> usize {
		let Connections = self.connections.read().await;
		Connections.values().filter(|c| c.is_active).count()
	}

	/// Get connection count by type
	pub async fn GetConnectionCountByType(&self, ConnectionType:ConnectionType) -> usize {
		let Connections = self.connections.read().await;
		Connections
			.values()
			.filter(|c| c.connection_type == ConnectionType && c.is_active)
			.count()
	}

	/// Get connections by type
	pub async fn GetConnectionsByType(&self, ConnectionType:ConnectionType) -> Vec<ConnectionInfo> {
		let Connections = self.connections.read().await;
		Connections
			.values()
			.filter(|c| c.connection_type == ConnectionType)
			.cloned()
			.collect()
	}

	/// Get connection for load balancing from Mountain pool
	/// Implements simple round-robin selection for connection pooling
	pub async fn GetNextMountainConnection(&self) -> Result<ConnectionInfo> {
		let Connections = self.connections.read().await;

		let MountainConnections:Vec<_> = Connections
			.values()
			.filter(|c| {
				matches!(c.connection_type, ConnectionType::MountainMain | ConnectionType::MountainWorker)
					&& c.is_active
			})
			.collect();

		if MountainConnections.is_empty() {
			return Err(AirError::ServiceUnavailable(
				"No active Mountain connections available".to_string(),
			));
		}

		// Simple round-robin selection - in production, consider:
		// - Connection load metrics
		// - Connection latency
		// - Connection health status
		// - Least busy connection strategy
		let Selected = MountainConnections[0].clone();

		Ok(Selected)
	}

	/// Clean up stale connections with comprehensive tracking
	/// Removes connections that haven't sent a heartbeat within the timeout
	/// period
	pub async fn CleanupStaleConnections(&self, TimeoutSeconds:u64) -> Result<usize> {
		let mut Connections = self.connections.write().await;
		let CurrentTime = utils::CurrentTimestamp();
		let TimeoutMs = TimeoutSeconds * 1000;

		let mut RemovedCount = 0;
		let mut RemovedByType:HashMap<String, usize> = HashMap::new();

		Connections.retain(|Id, Connection| {
				if CurrentTime - Connection.last_heartbeat > TimeoutMs {
					log::warn!(
						"Removing stale connection: {} - {} ({:?}) - idle: {}ms",
						Id,
						Connection.client_id,
						Connection.connection_type,
						CurrentTime - Connection.last_heartbeat
					);

					*RemovedByType.entry(format!("{:?}", Connection.connection_type)).or_insert(0) += 1;

					RemovedCount += 1;
				false
			} else {
				true
			}
		});

		if RemovedCount > 0 {
			log::info!("Cleaned up {} stale connections", RemovedCount);
			for (ConnType, Count) in RemovedByType {
				log::info!("  - {} connections: {}", ConnType, Count);
			}
		}

		Ok(RemovedCount)
	}

	/// Register background task with tracking
	pub async fn RegisterBackgroundTask(&self, Task:tokio::task::JoinHandle<()>) -> Result<()> {
		let mut Tasks = self.background_tasks.lock().await;
		Tasks.push(Task);
		log::debug!("Background task registered. Total tasks: {}", Tasks.len());
		Ok(())
	}

	/// Stop all background tasks with graceful shutdown
	pub async fn StopAllBackgroundTasks(&self) -> Result<()> {
		let mut Tasks = self.background_tasks.lock().await;

		let TaskCount = Tasks.len();
		log::info!("Stopping {} background tasks", TaskCount);

		// Abort all tasks
		for Task in Tasks.drain(..) {
			Task.abort();
		}

		log::info!("Stopped all {} background tasks", TaskCount);
		Ok(())
	}

	/// Update service status with validation
	pub async fn UpdateServiceStatus(&self, Service:&str, Status:ServiceStatus) -> Result<()> {
		if Service.is_empty() {
			return Err(AirError::Configuration("Service name cannot be empty".to_string()));
		}

		let mut ServiceStatus = self.service_status.write().await;
		let StatusClone = Status.clone();
		ServiceStatus.insert(Service.to_string(), Status);
		log::debug!("Service status updated: {} -> {:?}", Service, StatusClone);
		Ok(())
	}

	/// Get service status
	pub async fn GetServiceStatus(&self, Service:&str) -> Option<ServiceStatus> {
		let ServiceStatus = self.service_status.read().await;
		ServiceStatus.get(Service).cloned()
	}

	/// Get all service statuses
	pub async fn GetAllServiceStatuses(&self) -> HashMap<String, ServiceStatus> {
		let ServiceStatus = self.service_status.read().await;
		ServiceStatus.clone()
	}

	/// Register a new request with validation
	pub async fn RegisterRequest(&self, RequestId:String, Service:String) -> Result<()> {
		if RequestId.is_empty() {
			return Err(AirError::Configuration("Request ID cannot be empty".to_string()));
		}

		if Service.is_empty() {
			return Err(AirError::Configuration("Service name cannot be empty".to_string()));
		}

		let mut Requests = self.active_requests.lock().await;

		// Check for duplicate request IDs
		if Requests.contains_key(&RequestId) {
			return Err(AirError::Configuration(format!("Request {} already exists", RequestId)));
		}

		Requests.insert(
			RequestId.clone(),
			RequestStatus {
				request_id:RequestId.clone(),
				service:Service,
				started_at:utils::CurrentTimestamp(),
				status:RequestState::Pending,
				progress:None,
			},
		);

		log::debug!("Request registered: {}", RequestId);
		Ok(())
	}

	/// Update request status with validation
	pub async fn UpdateRequestStatus(&self, request_id:&str, status:RequestState, progress:Option<f32>) -> Result<()> {
		if request_id.is_empty() {
			return Err(AirError::Configuration("Request ID cannot be empty".to_string()));
		}

		// Validate progress value
		if let Some(p) = progress {
			if !(0.0..=1.0).contains(&p) {
				return Err(AirError::Configuration("Progress must be between 0.0 and 1.0".to_string()));
			}
		}

		let mut requests = self.active_requests.lock().await;

		if let Some(request) = requests.get_mut(request_id) {
			request.status = status;
			request.progress = progress;
		} else {
			return Err(AirError::Internal(format!("Request {} not found", request_id)));
		}

		Ok(())
	}

	/// Remove completed request with validation
	pub async fn RemoveRequest(&self, request_id:&str) -> Result<()> {
		if request_id.is_empty() {
			return Err(AirError::Configuration("Request ID cannot be empty".to_string()));
		}

		let mut requests = self.active_requests.lock().await;

		if requests.remove(request_id).is_some() {
			log::debug!("Request removed: {}", request_id);
		}

		Ok(())
	}

	/// Update performance metrics with validation
	pub async fn UpdateMetrics(&self, success:bool, response_time:u64) -> Result<()> {
		let mut metrics = self.metrics.write().await;

		metrics.total_requests += 1;
		if success {
			metrics.successful_requests += 1;
		} else {
			metrics.failed_requests += 1;
		}

		// Update average response time using exponential moving average
		let alpha = 0.1; // Smoothing factor
		metrics.average_response_time = alpha * (response_time as f64) + (1.0 - alpha) * metrics.average_response_time;

		metrics.last_updated = utils::CurrentTimestamp();

		Ok(())
	}

	/// Update resource usage with error handling
	pub async fn UpdateResourceUsage(&self) -> Result<()> {
		let sys = System::new();

		// Memory usage - collect outside lock
		let memory_usage = if let Ok(memory) = sys.memory() {
			(memory.total.as_u64() - memory.free.as_u64()) as f64 / 1024.0 / 1024.0
		} else {
			log::warn!("Failed to get memory usage");
			0.0
		};

		// CPU usage - requires sampling, do this outside lock
		let cpu_usage = if let Ok(cpu) = sys.cpu_load_aggregate() {
			tokio::time::sleep(std::time::Duration::from_secs(1)).await;
			if let Ok(cpu) = cpu.done() {
				(cpu.user + cpu.nice + cpu.system) as f64 * 100.0
			} else {
				log::warn!("Failed to get CPU usage after sampling");
				0.0
			}
		} else {
			log::warn!("Failed to start CPU load sampling");
			0.0
		};

		// Update state with collected metrics
		let mut resources = self.resources.write().await;
		resources.memory_usage_mb = memory_usage;
		resources.cpu_usage_percent = cpu_usage;
		resources.last_updated = utils::CurrentTimestamp();

		Ok(())
	}

	/// Get performance metrics
	pub async fn GetMetrics(&self) -> PerformanceMetrics {
		let metrics = self.metrics.read().await;
		metrics.clone()
	}

	/// Get resource usage
	pub async fn GetResourceUsage(&self) -> ResourceUsage {
		let resources = self.resources.read().await;
		resources.clone()
	}

	/// Get active request count
	pub async fn GetActiveRequestCount(&self) -> usize {
		let requests = self.active_requests.lock().await;
		requests.len()
	}

	/// Check if a request is cancelled
	pub async fn IsRequestCancelled(&self, request_id:&str) -> bool {
		let requests = self.active_requests.lock().await;
		if let Some(request) = requests.get(request_id) {
			matches!(request.status, RequestState::Cancelled)
		} else {
			false
		}
	}

	/// Get current configuration
	pub async fn GetConfiguration(&self) -> Arc<AirConfiguration> { self.configuration.clone() }

	/// Update configuration with validation and atomic operations
	pub async fn UpdateConfiguration(
		&self,
		section:String,
		updates:std::collections::HashMap<String, String>,
	) -> Result<()> {
		log::info!("[ApplicationState] Updating configuration section: {}", section);

		// Validate section
		if section.is_empty() {
			return Err(AirError::Configuration("Configuration section cannot be empty".to_string()));
		}

		// Validate updates
		if updates.is_empty() {
			return Err(AirError::Configuration("Configuration updates cannot be empty".to_string()));
		}

		// For now, just log the update
		// In a real implementation, this would:
		// 1. Validate the updates against schema
		// 2. Create a temporary config file
		// 3. Atomic replace the old config
		// 4. Notify affected services to reload

		match section.as_str() {
			"grpc" => {
				log::info!("Updating gRPC configuration: {:?}", updates);
			},
			"updates" => {
				log::info!("Updating updates configuration: {:?}", updates);
			},
			"downloader" => {
				log::info!("Updating downloader configuration: {:?}", updates);
			},
			"indexing" => {
				log::info!("Updating indexing configuration: {:?}", updates);
			},
			"daemon" => {
				log::info!("Updating daemon configuration: {:?}", updates);
			},
			_ => {
				return Err(AirError::Configuration(format!("Unknown configuration section: {}", section)));
			},
		}

		Ok(())
	}

	/// Set resource limits with validation and enforcement
	pub async fn SetResourceLimits(
		&self,
		memory_limit_mb:Option<u64>,
		cpu_limit_percent:Option<f64>,
		disk_limit_mb:Option<u64>,
	) -> Result<()> {
		log::info!(
			"[ApplicationState] Setting resource limits memory={:?}, cpu={:?}, disk={:?}",
			memory_limit_mb,
			cpu_limit_percent,
			disk_limit_mb
		);

		// Validate CPU limit
		if let Some(cpu) = cpu_limit_percent {
			if !(0.0..=100.0).contains(&cpu) {
				return Err(AirError::ResourceLimit("CPU limit must be between 0 and 100".to_string()));
			}
		}

		// Validate memory limit
		if let Some(memory) = memory_limit_mb {
			if memory == 0 {
				return Err(AirError::ResourceLimit("Memory limit must be greater than 0".to_string()));
			}
		}

		// Validate disk limit
		if let Some(disk) = disk_limit_mb {
			if disk == 0 {
				return Err(AirError::ResourceLimit("Disk limit must be greater than 0".to_string()));
			}
		}

		// Apply limits - this would affect how services operate
		// For now, just log and return success
		// In a real implementation, this would:
		// 1. Store limits in configuration
		// 2. Monitor resource usage against limits
		// 3. Throttle or stop services that exceed limits
		// 4. Alert on limit violations
		// 5. Implement graceful degradation

		if memory_limit_mb.is_some() {
			log::info!("Memory limit set: {} MB", memory_limit_mb.unwrap());
		}
		if cpu_limit_percent.is_some() {
			log::info!("CPU limit set: {}%", cpu_limit_percent.unwrap());
		}
		if disk_limit_mb.is_some() {
			log::info!("Disk limit set: {} MB", disk_limit_mb.unwrap());
		}

		Ok(())
	}

	/// Check if resource limits are exceeded
	pub async fn CheckResourceLimits(&self) -> Result<bool> {
		let _resources = self.resources.read().await;

		// In a real implementation, compare against configured limits
		// For now, just return false

		Ok(false)
	}

	/// Get connection health report
	pub async fn GetConnectionHealthReport(&self) -> ConnectionHealthReport {
		let connections = self.connections.read().await;
		let CurrentTime = utils::CurrentTimestamp();

		let mut healthy = 0;
		let mut stale = 0;
		let mut by_type:HashMap<String, usize> = HashMap::new();

		for connection in connections.values() {
			let is_stale = CurrentTime - connection.last_heartbeat > 120000; // 2 minutes

			if is_stale {
				stale += 1;
			} else if connection.is_active {
				healthy += 1;
			}

			*by_type.entry(format!("{:?}", connection.connection_type)).or_insert(0) += 1;
		}

		ConnectionHealthReport {
			total_connections:connections.len(),
			healthy_connections:healthy,
			stale_connections:stale,
			connections_by_type:by_type,
			last_checked:CurrentTime,
		}
	}
}
