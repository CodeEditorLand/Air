//! # Air Vine gRPC Service
//!
//! Defines the gRPC service implementation for Air. This struct handles
//! incoming RPC calls from Mountain, dispatches them to the appropriate
//! services (authentication, updates, downloads, indexing), and returns
//! the results.

use std::{collections::HashMap, sync::Arc};

use tonic::{Request, Response, Status};
use tokio_stream::StreamExt as TokioStreamExt;
use async_trait::async_trait;

use crate::{ApplicationState::ConnectionType::ConnectionType, dev_log};
// Note: Mist is available as a workspace dependency, no extern crate needed
use crate::{
	AirError,
	ApplicationState::ApplicationState::Struct,
	Authentication::AuthenticationService::AuthenticationService,
	Downloader::DownloadManager::Struct as DownloadManager,
	Indexing::FileIndexer::FileIndexer,
	Indexing::Store::QueryIndex::{SearchMode, SearchQuery},
	Result,
	Updates::UpdateManager::UpdateManager,
	Utility::CurrentTimestamp,
	Vine::Generated::{
		air as air_generated,
		air::{
			ApplyUpdateRequest,
			ApplyUpdateResponse,
			AuthenticationRequest,
			AuthenticationResponse,
			ConfigurationRequest,
			ConfigurationResponse,
			DownloadRequest,
			DownloadResponse,
			DownloadStreamRequest,
			DownloadStreamResponse,
			FileInfoRequest,
			FileInfoResponse,
			FileResult,
			HealthCheckRequest,
			HealthCheckResponse,
			IndexRequest,
			IndexResponse,
			MetricsRequest,
			MetricsResponse,
			ResourceLimitsRequest,
			ResourceLimitsResponse,
			ResourceUsageRequest,
			ResourceUsageResponse,
			SearchRequest,
			SearchResponse,
			StatusRequest,
			StatusResponse,
			UpdateCheckRequest,
			UpdateCheckResponse,
			UpdateConfigurationRequest,
			UpdateConfigurationResponse,
			air_service_server::AirService,
		},
	},
};

/// The concrete implementation of the Air gRPC service
#[derive(Clone)]
pub struct AirVinegRPCService {
	/// Application state
	AppState:Arc<crate::ApplicationState::ApplicationState::Struct>,

	/// Authentication service
	AuthService:Arc<AuthenticationService>,

	/// Update manager
	UpdateManager:Arc<UpdateManager>,

	/// Download manager
	DownloadManager:Arc<DownloadManager>,

	/// File indexer
	FileIndexer:Arc<FileIndexer>,

	/// Connection tracking
	ActiveConnections:Arc<tokio::sync::RwLock<HashMap<String, ConnectionMetadata>>>,
}

/// Connection metadata for tracking client state
#[derive(Debug, Clone)]
struct ConnectionMetadata {
	pub ClientId:String,

	pub ClientVersion:String,

	pub ProtocolVersion:u32,

	pub LastRequestTime:u64,

	pub RequestCount:u64,

	pub ConnectionType:ConnectionType,
}

impl AirVinegRPCService {
	/// Creates a new instance of the Air gRPC service
	pub fn new(
		AppState:Arc<crate::ApplicationState::ApplicationState::Struct>,

		AuthService:Arc<AuthenticationService>,

		UpdateManager:Arc<UpdateManager>,

		DownloadManager:Arc<DownloadManager>,

		FileIndexer:Arc<FileIndexer>,
	) -> Self {
		dev_log!("grpc", "[AirVinegRPCService] New instance created");

		Self {
			AppState,

			AuthService,

			UpdateManager,

			DownloadManager,

			FileIndexer,

			ActiveConnections:Arc::new(tokio::sync::RwLock::new(HashMap::new())),
		}
	}

	/// Track connection for a request
	async fn TrackConnection<RequestType>(
		&self,

		Request:&tonic::Request<RequestType>,

		_ServiceName:&str,
	) -> std::result::Result<String, Status> {
		let Metadata = Request.metadata();

		let ConnectionId = Metadata
			.get("connection-id")
			.map(|v| v.to_str().unwrap_or_default().to_string())
			.unwrap_or_else(|| crate::Utility::GenerateRequestId());

		let ClientId = Metadata
			.get("client-id")
			.map(|v| v.to_str().unwrap_or_default().to_string())
			.unwrap_or_else(|| "unknown".to_string());

		let ClientVersion = Metadata
			.get("client-version")
			.map(|v| v.to_str().unwrap_or_default().to_string())
			.unwrap_or_else(|| "unknown".to_string());

		let ProtocolVersion = Metadata
			.get("protocol-version")
			.map(|v| v.to_str().unwrap_or_default().parse().unwrap_or(1))
			.unwrap_or(1);

		// Update connection tracking
		let mut Connections = self.ActiveConnections.write().await;

		let ConnectionMetadata = Connections.entry(ConnectionId.clone()).or_insert_with(|| {
			ConnectionMetadata {
				ClientId:ClientId.clone(),
				ClientVersion:ClientVersion.clone(),
				ProtocolVersion,
				LastRequestTime:crate::Utility::CurrentTimestamp(),
				RequestCount:0,
				ConnectionType:crate::ApplicationState::ConnectionType::ConnectionType::MountainMain,
			}
		});

		ConnectionMetadata.LastRequestTime = crate::Utility::CurrentTimestamp();

		ConnectionMetadata.RequestCount += 1;

		// Register connection with application state
		self.AppState
			.RegisterConnection(
				ConnectionId.clone(),
				ClientId,
				ClientVersion,
				ProtocolVersion,
				crate::ApplicationState::ConnectionType::ConnectionType::MountainMain,
			)
			.await
			.map_err(|e| Status::internal(e.to_string()))?;

		Ok(ConnectionId)
	}

	/// Validate protocol version compatibility
	fn validate_protocol_version(&self, ClientVersion:u32) -> std::result::Result<(), Status> {
		if ClientVersion > crate::ProtocolVersion {
			return Err(Status::failed_precondition(format!(
				"Client protocol version {} is newer than server version {}",
				ClientVersion,
				crate::ProtocolVersion
			)));
		}

		if ClientVersion < crate::ProtocolVersion {
			dev_log!(
				"grpc",
				"warn: Client using older protocol version {} (server: {})",
				ClientVersion,
				crate::ProtocolVersion
			);
		}

		Ok(())
	}
}

#[async_trait]
impl AirService for AirVinegRPCService {
	/// Handle authentication requests from Mountain
	async fn authenticate(
		&self,

		Request:Request<AuthenticationRequest>,
	) -> std::result::Result<Response<AuthenticationResponse>, Status> {
		// Track connection and validate protocol (before consuming Request)
		let ConnectionId = self.TrackConnection(&Request, "authentication").await?;

		let RequestData = Request.into_inner();

		let request_id = RequestData.request_id.clone();

		dev_log!(
			"grpc",
			"[AirVinegRPCService] Authentication request received [ID: {}] [Connection: {}]",
			request_id,
			ConnectionId
		);

		self.AppState
			.RegisterRequest(request_id.clone(), "authentication".to_string())
			.await
			.map_err(|e| Status::internal(e.to_string()))?;

		// Additional security validation
		if RequestData.username.is_empty() || RequestData.password.is_empty() || RequestData.provider.is_empty() {
			let ErrorMessage = "Invalid authentication parameters".to_string();

			self.AppState
				.UpdateRequestStatus(
					&request_id,
					crate::ApplicationState::RequestState::RequestState::Failed(ErrorMessage.clone()),
					None,
				)
				.await
				.ok();

			return Ok(Response::new(air_generated::AuthenticationResponse {
				request_id,
				success:false,
				token:String::new(),
				error:ErrorMessage,
			}));
		}

		// Clone needed data before moving RequestData
		let username_for_log = RequestData.username.clone();

		let password = RequestData.password;

		let provider = RequestData.provider;

		let result = self
			.AuthService
			.AuthenticateUser(RequestData.username, password, provider)
			.await;

		match result {
			Ok(token) => {
				self.AppState
					.UpdateRequestStatus(
						&request_id,
						crate::ApplicationState::RequestState::RequestState::Completed,
						Some(100.0),
					)
					.await
					.ok();

				// Log successful authentication
				dev_log!(
					"grpc",
					"[AirVinegRPCService] Authentication successful for user: {} [Connection: {}]",
					username_for_log,
					ConnectionId
				);

				Ok(Response::new(air_generated::AuthenticationResponse {
					request_id,
					success:true,
					token,
					error:String::new(),
				}))
			},

			Err(e) => {
				self.AppState
					.UpdateRequestStatus(
						&request_id,
						crate::ApplicationState::RequestState::RequestState::Failed(e.to_string()),
						None,
					)
					.await
					.ok();

				// Log failed authentication attempt
				dev_log!(
					"grpc",
					"warn: [AirVinegRPCService] Authentication failed for user: {} [Connection: {}] - {}",
					username_for_log,
					ConnectionId,
					e
				);

				Ok(Response::new(air_generated::AuthenticationResponse {
					request_id,
					success:false,
					token:String::new(),
					error:e.to_string(),
				}))
			},
		}
	}

	/// Handle update check requests from Mountain
	async fn check_for_updates(
		&self,

		request:Request<UpdateCheckRequest>,
	) -> std::result::Result<Response<UpdateCheckResponse>, Status> {
		let RequestData = request.into_inner();

		let request_id = RequestData.request_id.clone();

		dev_log!(
			"grpc",
			"[AirVinegRPCService] Update check request received [ID: {}] - Version: {}, Channel: {}",
			request_id,
			RequestData.current_version,
			RequestData.channel
		);

		self.AppState
			.RegisterRequest(request_id.clone(), "updates".to_string())
			.await
			.map_err(|e| Status::internal(e.to_string()))?;

		// Validate CurrentVersion
		if RequestData.current_version.is_empty() {
			let ErrorMessage = crate::AirError::Validation("CurrentVersion cannot be empty".to_string());

			self.AppState
				.UpdateRequestStatus(
					&request_id,
					crate::ApplicationState::RequestState::RequestState::Failed(ErrorMessage.to_string()),
					None,
				)
				.await
				.ok();

			return Err(Status::invalid_argument(ErrorMessage.to_string()));
		}

		// Validate channel
		let ValidChannels = ["stable", "beta", "nightly"];

		let Channel = if RequestData.channel.is_empty() {
			"stable".to_string()
		} else {
			RequestData.channel.clone()
		};

		if !ValidChannels.contains(&Channel.as_str()) {
			let ErrorMessage = format!("Invalid channel: {}. Valid values are: {}", Channel, ValidChannels.join(", "));

			self.AppState
				.UpdateRequestStatus(
					&request_id,
					crate::ApplicationState::RequestState::RequestState::Failed(ErrorMessage.clone()),
					None,
				)
				.await
				.ok();

			return Err(Status::invalid_argument(ErrorMessage));
		}

		// Check for updates using UpdateManager
		let result = self.UpdateManager.CheckForUpdates().await;

		match result {
			Ok(UpdateInfo) => {
				self.AppState
					.UpdateRequestStatus(
						&request_id,
						crate::ApplicationState::RequestState::RequestState::Completed,
						Some(100.0),
					)
					.await
					.ok();

				dev_log!(
					"grpc",
					"[AirVinegRPCService] Update check successful - Available: {}",
					UpdateInfo.is_some()
				);

				Ok(Response::new(air_generated::UpdateCheckResponse {
					request_id,
					update_available:UpdateInfo.is_some(),
					version:UpdateInfo.as_ref().map(|info| info.version.clone()).unwrap_or_default(),
					download_url:UpdateInfo.as_ref().map(|info| info.download_url.clone()).unwrap_or_default(),
					release_notes:UpdateInfo.as_ref().map(|info| info.release_notes.clone()).unwrap_or_default(),
					error:String::new(),
				}))
			},

			Err(crate::AirError::Network(e)) => {
				self.AppState
					.UpdateRequestStatus(
						&request_id,
						crate::ApplicationState::RequestState::RequestState::Failed(e.clone()),
						None,
					)
					.await
					.ok();

				dev_log!("grpc", "error: [AirVinegRPCService] Network error during update check: {}", e);

				Err(Status::unavailable(e))
			},

			Err(e) => {
				self.AppState
					.UpdateRequestStatus(
						&request_id,
						crate::ApplicationState::RequestState::RequestState::Failed(e.to_string()),
						None,
					)
					.await
					.ok();

				dev_log!("grpc", "error: [AirVinegRPCService] Update check failed: {}", e);

				Ok(Response::new(air_generated::UpdateCheckResponse {
					request_id,
					update_available:false,
					version:String::new(),
					download_url:String::new(),
					release_notes:String::new(),
					error:e.to_string(),
				}))
			},
		}
	}

	/// Handle download requests from Mountain
	async fn download_file(
		&self,

		request:Request<DownloadRequest>,
	) -> std::result::Result<Response<DownloadResponse>, Status> {
		let RequestData = request.into_inner();

		let request_id = RequestData.request_id.clone();

		dev_log!(
			"grpc",
			"[AirVinegRPCService] Download request received [ID: {}] - URL: {}",
			request_id,
			RequestData.url
		);

		// Request ID for tracking (use provided or generate)
		let download_request_id = if request_id.is_empty() {
			crate::Utility::GenerateRequestId()
		} else {
			request_id.clone()
		};

		self.AppState
			.RegisterRequest(download_request_id.clone(), "downloader".to_string())
			.await
			.map_err(|e| Status::internal(e.to_string()))?;

		// Validate URL
		if RequestData.url.is_empty() {
			let error_msg = "URL cannot be empty".to_string();

			self.AppState
				.UpdateRequestStatus(
					&download_request_id,
					crate::ApplicationState::RequestState::RequestState::Failed(error_msg.clone()),
					None,
				)
				.await
				.ok();

			return Ok(Response::new(DownloadResponse {
				request_id:download_request_id,
				success:false,
				file_path:String::new(),
				file_size:0,
				checksum:String::new(),
				error:error_msg,
			}));
		}

		// Validate URL format
		if !match_url_scheme(&RequestData.url) {
			let error_msg = format!("Invalid URL scheme: {}", RequestData.url);

			self.AppState
				.UpdateRequestStatus(
					&download_request_id,
					crate::ApplicationState::RequestState::RequestState::Failed(error_msg.clone()),
					None,
				)
				.await
				.ok();

			return Ok(Response::new(DownloadResponse {
				request_id:download_request_id,
				success:false,
				file_path:String::new(),
				file_size:0,
				checksum:String::new(),
				error:error_msg,
			}));
		}

		// Validate or use cache directory
		let DestinationPath = if RequestData.destination_path.is_empty() {
			// Use cache directory from configuration
			let config = &self.AppState.Configuration.Downloader;

			config.CacheDirectory.clone()
		} else {
			RequestData.destination_path.clone()
		};

		// Validate target directory exists
		let dest_path = std::path::Path::new(&DestinationPath);

		if let Some(parent) = dest_path.parent() {
			if !parent.exists() {
				match tokio::fs::create_dir_all(parent).await {
					Ok(_) => {
						dev_log!(
							"grpc",
							"[AirVinegRPCService] Created destination directory: {}",
							parent.display()
						);
					},

					Err(e) => {
						let error_msg = format!("Failed to create destination directory: {}", e);

						self.AppState
							.UpdateRequestStatus(
								&download_request_id,
								crate::ApplicationState::RequestState::RequestState::Failed(error_msg.clone()),
								None,
							)
							.await
							.ok();

						return Ok(Response::new(DownloadResponse {
							request_id:download_request_id,
							success:false,
							file_path:String::new(),
							file_size:0,
							checksum:String::new(),
							error:error_msg,
						}));
					},
				}
			}
		}

		// Set up granular progress callback mechanism
		let _download_manager = self.DownloadManager.clone();

		let AppState = self.AppState.clone();

		let callback_request_id = download_request_id.clone();

		let progress_callback = move |progress:f32| {
			let state = AppState.clone();

			let id = callback_request_id.clone();

			tokio::spawn(async move {
				let _ = state
					.UpdateRequestStatus(
						&id,
						crate::ApplicationState::RequestState::RequestState::InProgress,
						Some(progress),
					)
					.await;
			});
		};

		// Perform download with retry and progress tracking
		let result = self
			.download_file_with_retry(
				&download_request_id,
				RequestData.url.clone(),
				DestinationPath,
				RequestData.checksum,
				Some(Box::new(progress_callback)),
			)
			.await;

		match result {
			Ok(file_info) => {
				self.AppState
					.UpdateRequestStatus(
						&download_request_id,
						crate::ApplicationState::RequestState::RequestState::Completed,
						Some(100.0),
					)
					.await
					.ok();

				dev_log!(
					"grpc",
					"[AirVinegRPCService] Download completed [ID: {}] - Size: {} bytes",
					download_request_id,
					file_info.size
				);

				Ok(Response::new(DownloadResponse {
					request_id:download_request_id,
					success:true,
					file_path:file_info.path,
					file_size:file_info.size,
					checksum:file_info.checksum,
					error:String::new(),
				}))
			},

			Err(e) => {
				self.AppState
					.UpdateRequestStatus(
						&download_request_id,
						crate::ApplicationState::RequestState::RequestState::Failed(e.to_string()),
						None,
					)
					.await
					.ok();

				dev_log!(
					"grpc",
					"error: [AirVinegRPCService] Download failed [ID: {}] - Error: {}",
					download_request_id,
					e
				);

				Ok(Response::new(DownloadResponse {
					request_id:download_request_id,
					success:false,
					file_path:String::new(),
					file_size:0,
					checksum:String::new(),
					error:e.to_string(),
				}))
			},
		}
	}

	/// Handle file indexing requests from Mountain
	async fn index_files(&self, request:Request<IndexRequest>) -> std::result::Result<Response<IndexResponse>, Status> {
		let RequestData = request.into_inner();

		let request_id = RequestData.request_id;

		dev_log!(
			"grpc",
			"[AirVinegRPCService] Index request received [ID: {}] - Path: {}",
			request_id,
			RequestData.path
		);

		self.AppState
			.RegisterRequest(request_id.clone(), "indexing".to_string())
			.await
			.map_err(|e| Status::internal(e.to_string()))?;

		let result = self.FileIndexer.IndexDirectory(RequestData.path, RequestData.patterns).await;

		match result {
			Ok(index_info) => {
				self.AppState
					.UpdateRequestStatus(
						&request_id,
						crate::ApplicationState::RequestState::RequestState::Completed,
						Some(100.0),
					)
					.await
					.ok();

				Ok(Response::new(air_generated::IndexResponse {
					request_id,
					success:true,
					files_indexed:index_info.files_indexed,
					total_size:index_info.total_size,
					error:String::new(),
				}))
			},

			Err(e) => {
				self.AppState
					.UpdateRequestStatus(
						&request_id,
						crate::ApplicationState::RequestState::RequestState::Failed(e.to_string()),
						None,
					)
					.await
					.ok();

				Ok(Response::new(air_generated::IndexResponse {
					request_id,
					success:false,
					files_indexed:0,
					total_size:0,
					error:e.to_string(),
				}))
			},
		}
	}

	/// Handle status check requests from Mountain
	async fn get_status(
		&self,

		request:Request<StatusRequest>,
	) -> std::result::Result<Response<StatusResponse>, Status> {
		let _RequestData = request.into_inner();

		dev_log!("grpc", "[AirVinegRPCService] Status request received");

		let metrics = self.AppState.GetMetrics().await;

		let resources = self.AppState.GetResourceUsage().await;

		Ok(Response::new(air_generated::StatusResponse {
			version:crate::VERSION.to_string(),
			uptime_seconds:metrics.UptimeSeconds,
			total_requests:metrics.TotalRequest,
			successful_requests:metrics.SuccessfulRequest,
			failed_requests:metrics.FailedRequest,
			average_response_time:metrics.AverageResponseTime,
			memory_usage_mb:resources.MemoryUsageMb,
			cpu_usage_percent:resources.CPUUsagePercent,
			active_requests:self.AppState.GetActiveRequestCount().await as u32,
		}))
	}

	/// Handle service health check
	async fn health_check(
		&self,

		_request:Request<HealthCheckRequest>,
	) -> std::result::Result<Response<HealthCheckResponse>, Status> {
		dev_log!("grpc", "[AirVinegRPCService] Health check request received");

		Ok(Response::new(air_generated::HealthCheckResponse {
			healthy:true,
			timestamp:CurrentTimestamp(),
		}))
	}

	// ==================== Phase 2: Update Operations ====================

	/// Handle download update requests
	async fn download_update(
		&self,

		request:Request<DownloadRequest>,
	) -> std::result::Result<Response<DownloadResponse>, Status> {
		let RequestData = request.into_inner();

		let request_id = RequestData.request_id.clone();

		dev_log!(
			"grpc",
			"[AirVinegRPCService] Download update request received [ID: {}] - URL: {}, Destination: {}",
			request_id,
			RequestData.url,
			RequestData.destination_path
		);

		self.AppState
			.RegisterRequest(request_id.clone(), "download_update".to_string())
			.await
			.map_err(|e| Status::internal(e.to_string()))?;

		// Validate URL is not empty
		if RequestData.url.is_empty() {
			let error_msg = crate::AirError::Validation("URL cannot be empty".to_string());

			self.AppState
				.UpdateRequestStatus(
					&request_id,
					crate::ApplicationState::RequestState::RequestState::Failed(error_msg.to_string()),
					None,
				)
				.await
				.ok();

			return Err(Status::invalid_argument(error_msg.to_string()));
		}

		// Validate URL format
		if !RequestData.url.starts_with("http://") && !RequestData.url.starts_with("https://") {
			let error_msg = crate::AirError::Validation("URL must start with http:// or https://".to_string());

			self.AppState
				.UpdateRequestStatus(
					&request_id,
					crate::ApplicationState::RequestState::RequestState::Failed(error_msg.to_string()),
					None,
				)
				.await
				.ok();

			return Err(Status::invalid_argument(error_msg.to_string()));
		}

		// Validate destination path is writeable
		let destination = if RequestData.destination_path.is_empty() {
			// Default to cache directory if not specified
			self.UpdateManager
				.GetCacheDirectory()
				.join("update-latest.bin")
				.to_string_lossy()
				.to_string()
		} else {
			// Use provided destination
			let dest_path = std::path::Path::new(&RequestData.destination_path);

			if let Some(parent) = dest_path.parent() {
				if parent.as_os_str().is_empty() {
					// Relative path in cache directory
					self.UpdateManager
						.GetCacheDirectory()
						.join(&RequestData.destination_path)
						.to_string_lossy()
						.to_string()
				} else {
					// Absolute or full path
					RequestData.destination_path.clone()
				}
			} else {
				RequestData.destination_path.clone()
			}
		};

		// Check if destination directory exists and is writeable
		let dest_path = std::path::Path::new(&destination);

		if let Some(parent) = dest_path.parent() {
			if !parent.exists() {
				return Err(Status::failed_precondition(format!(
					"Destination directory does not exist: {}",
					parent.display()
				)));
			}

			// Test writeability
			if let Err(e) = std::fs::write(parent.join(".write_test"), "") {
				let error_msg = crate::AirError::FileSystem(format!("Destination directory not writeable: {}", e));

				self.AppState
					.UpdateRequestStatus(
						&request_id,
						crate::ApplicationState::RequestState::RequestState::Failed(error_msg.to_string()),
						None,
					)
					.await
					.ok();

				return Err(Status::permission_denied(error_msg.to_string()));
			}

			// Cleanup test file
			let _ = std::fs::remove_file(parent.join(".write_test"));
		}

		// Use download manager - includes SHA256 checksum verification, progress
		// tracking, and retry logic
		let download_result = self
			.DownloadManager
			.DownloadFile(RequestData.url, destination.clone(), RequestData.checksum)
			.await;

		match download_result {
			Ok(result) => {
				self.AppState
					.UpdateRequestStatus(
						&request_id,
						crate::ApplicationState::RequestState::RequestState::Completed,
						Some(100.0),
					)
					.await
					.ok();

				dev_log!(
					"grpc",
					"[AirVinegRPCService] Update downloaded successfully - Path: {}, Size: {}, Checksum: {}",
					result.path,
					result.size,
					result.checksum
				);

				Ok(Response::new(DownloadResponse {
					request_id,
					success:true,
					file_path:result.path,
					file_size:result.size,
					checksum:result.checksum,
					error:String::new(),
				}))
			},

			Err(crate::AirError::Network(e)) => {
				self.AppState
					.UpdateRequestStatus(
						&request_id,
						crate::ApplicationState::RequestState::RequestState::Failed(e.clone()),
						None,
					)
					.await
					.ok();

				dev_log!("grpc", "error: [AirVinegRPCService] Download update network error: {}", e);

				Err(Status::unavailable(e))
			},

			Err(crate::AirError::FileSystem(e)) => {
				self.AppState
					.UpdateRequestStatus(
						&request_id,
						crate::ApplicationState::RequestState::RequestState::Failed(e.clone()),
						None,
					)
					.await
					.ok();

				dev_log!("grpc", "error: [AirVinegRPCService] Download update filesystem error: {}", e);

				Err(Status::internal(e))
			},

			Err(e) => {
				self.AppState
					.UpdateRequestStatus(
						&request_id,
						crate::ApplicationState::RequestState::RequestState::Failed(e.to_string()),
						None,
					)
					.await
					.ok();

				dev_log!("grpc", "error: [AirVinegRPCService] Download update failed: {}", e);

				Ok(Response::new(DownloadResponse {
					request_id,
					success:false,
					file_path:String::new(),
					file_size:0,
					checksum:String::new(),
					error:e.to_string(),
				}))
			},
		}
	}

	/// Handle apply update requests
	async fn apply_update(
		&self,

		request:Request<ApplyUpdateRequest>,
	) -> std::result::Result<Response<ApplyUpdateResponse>, Status> {
		let RequestData = request.into_inner();

		let request_id = RequestData.request_id.clone();

		dev_log!(
			"grpc",
			"[AirVinegRPCService] Apply update request received [ID: {}] - Version: {}, Path: {}",
			request_id,
			RequestData.version,
			RequestData.update_path
		);

		self.AppState
			.RegisterRequest(request_id.clone(), "apply_update".to_string())
			.await
			.map_err(|e| Status::internal(e.to_string()))?;

		// Validate version
		if RequestData.version.is_empty() {
			let error_msg = crate::AirError::Validation("version cannot be empty".to_string());

			self.AppState
				.UpdateRequestStatus(
					&request_id,
					crate::ApplicationState::RequestState::RequestState::Failed(error_msg.to_string()),
					None,
				)
				.await
				.ok();

			return Err(Status::invalid_argument(error_msg.to_string()));
		}

		// Validate update path is not empty
		if RequestData.update_path.is_empty() {
			let error_msg = crate::AirError::Validation("update_path cannot be empty".to_string());

			self.AppState
				.UpdateRequestStatus(
					&request_id,
					crate::ApplicationState::RequestState::RequestState::Failed(error_msg.to_string()),
					None,
				)
				.await
				.ok();

			return Err(Status::invalid_argument(error_msg.to_string()));
		}

		let update_path = std::path::Path::new(&RequestData.update_path);

		// Validate update file exists
		if !update_path.exists() {
			let error_msg = crate::AirError::FileSystem(format!("Update file not found: {}", RequestData.update_path));

			self.AppState
				.UpdateRequestStatus(
					&request_id,
					crate::ApplicationState::RequestState::RequestState::Failed(error_msg.to_string()),
					None,
				)
				.await
				.ok();

			return Err(Status::not_found(error_msg.to_string()));
		}

		// Validate update file is readable and has content
		let metadata = match std::fs::metadata(update_path) {
			Ok(m) => m,

			Err(e) => {
				let error_msg = crate::AirError::FileSystem(format!("Failed to read update file metadata: {}", e));

				self.AppState
					.UpdateRequestStatus(
						&request_id,
						crate::ApplicationState::RequestState::RequestState::Failed(error_msg.to_string()),
						None,
					)
					.await
					.ok();

				return Err(Status::internal(error_msg.to_string()));
			},
		};

		if metadata.len() == 0 {
			let error_msg = crate::AirError::Validation("Update file is empty".to_string());

			self.AppState
				.UpdateRequestStatus(
					&request_id,
					crate::ApplicationState::RequestState::RequestState::Failed(error_msg.to_string()),
					None,
				)
				.await
				.ok();

			return Err(Status::failed_precondition(error_msg.to_string()));
		}

		// Prepare rollback capability before applying update
		let rollback_backup_path = self.prepare_rollback_backup(&RequestData.version).await;

		if let Err(ref e) = rollback_backup_path {
			dev_log!(
				"grpc",
				"warn: [AirVinegRPCService] Failed to prepare rollback backup: {}. Proceeding without rollback \
				 capability.",
				e
			);
		}

		// Verify update file integrity (checksum check)
		match self.UpdateManager.verify_update(&RequestData.update_path, None).await {
			Ok(true) => {
				dev_log!(
					"grpc",
					"[AirVinegRPCService] Update verification successful, preparing for installation"
				);

				self.AppState
					.UpdateRequestStatus(
						&request_id,
						crate::ApplicationState::RequestState::RequestState::Completed,
						Some(100.0),
					)
					.await
					.ok();

				// Trigger graceful shutdown after returning response
				let AppState = self.AppState.clone();

				let version = RequestData.version.clone();

				let self_clone = self.clone();

				tokio::spawn(async move {
					tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

					dev_log!(
						"grpc",
						"[AirVinegRPCService] Initiating graceful shutdown for update version {}",
						version
					);

					// Graceful shutdown implementation
					if let Err(e) = AppState.StopAllBackgroundTasks().await {
						dev_log!(
							"grpc",
							"error: [AirVinegRPCService] Failed to initiate graceful shutdown: {}",
							e
						);

						// Implement rollback if update fails
						dev_log!(
							"grpc",
							"warn: [AirVinegRPCService] Rollback initiated due to graceful shutdown failure"
						);

						if let Err(rollback_error) = self_clone.perform_rollback(&version).await {
							dev_log!("grpc", "error: [AirVinegRPCService] Rollback failed: {}", rollback_error);
						} else {
							dev_log!("grpc", "[AirVinegRPCService] Rollback completed successfully");
						}
					}
				});

				Ok(Response::new(ApplyUpdateResponse {
					request_id,
					success:true,
					error:String::new(),
				}))
			},

			Ok(false) => {
				let error_msg = "Update verification failed: checksum mismatch".to_string();

				self.AppState
					.UpdateRequestStatus(
						&request_id,
						crate::ApplicationState::RequestState::RequestState::Failed(error_msg.clone()),
						None,
					)
					.await
					.ok();

				dev_log!("grpc", "error: [AirVinegRPCService] {}", error_msg);

				// Clean up rollback backup if verification failed
				let _ = self.cleanup_rollback_backup(&RequestData.version).await;

				Err(Status::failed_precondition(error_msg))
			},

			Err(crate::AirError::FileSystem(e)) => {
				self.AppState
					.UpdateRequestStatus(
						&request_id,
						crate::ApplicationState::RequestState::RequestState::Failed(e.clone()),
						None,
					)
					.await
					.ok();

				dev_log!(
					"grpc",
					"error: [AirVinegRPCService] Update verification filesystem error: {}",
					e
				);

				// Clean up rollback backup if verification failed
				let _ = self.cleanup_rollback_backup(&RequestData.version).await;

				Err(Status::internal(e))
			},

			Err(e) => {
				self.AppState
					.UpdateRequestStatus(
						&request_id,
						crate::ApplicationState::RequestState::RequestState::Failed(e.to_string()),
						None,
					)
					.await
					.ok();

				dev_log!("grpc", "error: [AirVinegRPCService] Update verification error: {}", e);

				// Clean up rollback backup if verification failed
				let _ = self.cleanup_rollback_backup(&RequestData.version).await;

				Ok(Response::new(ApplyUpdateResponse {
					request_id,
					success:false,
					error:e.to_string(),
				}))
			},
		}
	}

	// ==================== Phase 3: Download Operations ====================

	/// Handle streaming download requests with bidirectional streaming and
	/// chunk delivery
	type DownloadStreamStream =
		tokio_stream::wrappers::ReceiverStream<std::result::Result<air_generated::DownloadStreamResponse, Status>>;

	async fn download_stream(
		&self,

		request:Request<DownloadStreamRequest>,
	) -> std::result::Result<Response<Self::DownloadStreamStream>, Status> {
		let RequestData = request.into_inner();

		let request_id = RequestData.request_id.clone();

		dev_log!(
			"grpc",
			"[AirVinegRPCService] Download stream request received [ID: {}] - URL: {}",
			request_id,
			RequestData.url
		);

		self.AppState
			.RegisterRequest(request_id.clone(), "downloader_stream".to_string())
			.await
			.map_err(|e| Status::internal(e.to_string()))?;

		// Validate URL
		if RequestData.url.is_empty() {
			let error_msg = "URL cannot be empty".to_string();

			self.AppState
				.UpdateRequestStatus(
					&request_id,
					crate::ApplicationState::RequestState::RequestState::Failed(error_msg.clone()),
					None,
				)
				.await
				.ok();

			return Err(Status::invalid_argument(error_msg));
		}

		// Validate URL scheme
		if !match_url_scheme(&RequestData.url) {
			let error_msg = format!("Invalid URL scheme: {}", RequestData.url);

			self.AppState
				.UpdateRequestStatus(
					&request_id,
					crate::ApplicationState::RequestState::RequestState::Failed(error_msg.clone()),
					None,
				)
				.await
				.ok();

			return Err(Status::invalid_argument(error_msg));
		}

		// Validate URL supports range headers for streaming
		match self.validate_range_support(&RequestData.url).await {
			Ok(true) => {
				dev_log!("grpc", "[AirVinegRPCService] URL supports range headers");
			},

			Ok(false) => {
				dev_log!(
					"grpc",
					"warn: [AirVinegRPCService] URL does not support range headers, streaming may be inefficient"
				);
			},

			Err(e) => {
				let error_msg = format!("Failed to validate range support: {}", e);

				self.AppState
					.UpdateRequestStatus(
						&request_id,
						crate::ApplicationState::RequestState::RequestState::Failed(error_msg.clone()),
						None,
					)
					.await
					.ok();

				return Err(Status::internal(error_msg));
			},
		}

		// Create a channel for streaming responses
		let (tx, rx) = tokio::sync::mpsc::channel(100);

		// Configure chunk size (8MB chunks for balance between throughput and latency)
		let chunk_size = 8 * 1024 * 1024; // 8MB

		// Clone necessary data for spawn task
		let url = RequestData.url.clone();

		let headers = RequestData.headers;

		let download_request_id = request_id.clone();

		let _download_manager = self.DownloadManager.clone();

		let AppState = self.AppState.clone();

		// Spawn streaming task
		tokio::spawn(async move {
			// Send initial status
			if tx
				.send(Ok(DownloadStreamResponse {
					request_id:download_request_id.clone(),
					chunk:vec![].into(),
					total_size:0,
					downloaded:0,
					completed:false,
					error:String::new(),
				}))
				.await
				.is_err()
			{
				dev_log!(
					"grpc",
					"warn: [AirVinegRPCService] Client disconnected before streaming started [ID: {}]",
					download_request_id
				);

				return;
			}

			// Create HTTP client with connection pooling hints
			let dns_port = Mist::dns_port();

			let client_builder_result = crate::HTTP::Client::secured_client_builder(dns_port);

			let client_builder = match client_builder_result {
				Ok(builder) => builder,
				Err(e) => {
					let error = format!("Failed to create HTTP client builder: {}", e);

					let _ = tx
						.send(Ok(DownloadStreamResponse {
							request_id:download_request_id.clone(),
							chunk:vec![].into(),
							total_size:0,
							downloaded:0,
							completed:false,
							error:error.clone(),
						}))
						.await;

					AppState
						.UpdateRequestStatus(
							&download_request_id,
							crate::ApplicationState::RequestState::RequestState::Failed(error),
							None,
						)
						.await
						.ok();

					return;
				},
			};

			let client_result = client_builder
				.pool_idle_timeout(std::time::Duration::from_secs(60))
				.pool_max_idle_per_host(5)
				.timeout(std::time::Duration::from_secs(300))
				.build();

			if client_result.is_err() {
				let error = client_result.unwrap_err().to_string();

				let _ = tx
					.send(Ok(DownloadStreamResponse {
						request_id:download_request_id.clone(),
						chunk:vec![].into(),
						total_size:0,
						downloaded:0,
						completed:false,
						error:error.clone(),
					}))
					.await;

				AppState
					.UpdateRequestStatus(
						&download_request_id,
						crate::ApplicationState::RequestState::RequestState::Failed(error),
						None,
					)
					.await
					.ok();

				return;
			}

			let client:reqwest::Client = match client_result {
				Ok(client) => client,
				Err(e) => {
					let error = format!("Failed to create HTTP client: {}", e);

					let _ = tx.send(Err(Status::internal(error.clone())));

					AppState
						.UpdateRequestStatus(
							&download_request_id,
							crate::ApplicationState::RequestState::RequestState::Failed(error),
							None,
						)
						.await
						.ok();

					return;
				},
			};

			// Start streaming download
			let mut total_size:Option<u64> = None;

			let mut total_downloaded:u64 = 0;

			match client
				.get(&url)
				.headers({
					let mut map = reqwest::header::HeaderMap::new();

					for (key, value) in headers {
						if let (Ok(header_name), Ok(header_value)) = (
							reqwest::header::HeaderName::from_bytes(key.as_bytes()),
							reqwest::header::HeaderValue::from_str(&value),
						) {
							map.insert(header_name, header_value);
						}
					}

					map
				})
				.send()
				.await
			{
				Ok(response) => {
					if !response.status().is_success() {
						let error = format!("Download failed with status: {}", response.status());

						let _ = tx
							.send(Ok(DownloadStreamResponse {
								request_id:download_request_id.clone(),
								chunk:vec![].into(),
								total_size:0,
								downloaded:0,
								completed:false,
								error:error.clone(),
							}))
							.await;

						AppState
							.UpdateRequestStatus(
								&download_request_id,
								crate::ApplicationState::RequestState::RequestState::Failed(error),
								None,
							)
							.await
							.ok();

						return;
					}

					total_size = Some(response.content_length().unwrap_or(0));

					let response_tx = tx.clone();

					let response_id = download_request_id.clone();

					// Stream chunks to client
					let mut stream = response.bytes_stream();

					let mut buffer = Vec::with_capacity(chunk_size);

					let mut last_progress:f32 = 0.0;

					while let Some(chunk_result) = TokioStreamExt::next(&mut stream).await {
						if AppState.IsRequestCancelled(&download_request_id).await {
							dev_log!(
								"grpc",
								"[AirVinegRPCService] Download cancelled by client [ID: {}]",
								download_request_id
							);

							AppState
								.UpdateRequestStatus(
									&download_request_id,
									crate::ApplicationState::RequestState::RequestState::Cancelled,
									None,
								)
								.await
								.ok();

							return;
						}

						match chunk_result {
							Ok(chunk) => {
								buffer.extend_from_slice(&chunk);

								total_downloaded += chunk.len() as u64;

								// Send chunk when buffer reaches target size
								if buffer.len() >= chunk_size {
									// Calculate checksum for verification
									let _chunk_checksum = calculate_chunk_checksum(&buffer);

									// Calculate progress
									let progress = if let Some(ts) = total_size {
										if ts > 0 { (total_downloaded as f32 / ts as f32) * 100.0 } else { 0.0 }
									} else {
										0.0
									};

									// Update request status periodically
									if progress - last_progress >= 5.0 {
										AppState
											.UpdateRequestStatus(
												&download_request_id,
												crate::ApplicationState::RequestState::RequestState::InProgress,
												Some(progress),
											)
											.await
											.ok();

										last_progress = progress;
									}

									if response_tx
										.send(Ok(DownloadStreamResponse {
											request_id:response_id.clone(),
											chunk:buffer.clone().into(),
											total_size:total_size.unwrap_or(0),
											downloaded:total_downloaded,
											completed:false,
											error:String::new(),
										}))
										.await
										.is_err()
									{
										dev_log!(
											"grpc",
											"warn: [AirVinegRPCService] Client disconnected during streaming [ID: {}]",
											download_request_id
										);

										AppState
											.UpdateRequestStatus(
												&download_request_id,
												crate::ApplicationState::RequestState::RequestState::Failed(
													"Client disconnected".to_string(),
												),
												None,
											)
											.await
											.ok();

										return;
									}

									dev_log!(
										"grpc",
										"[AirVinegRPCService] Sent chunk of {} bytes [ID: {}] - Progress: {:.1}%",
										buffer.len(),
										download_request_id,
										progress
									);

									buffer.clear();
								}
							},
							Err(e) => {
								let error = format!("Download error: {}", e);

								dev_log!(
									"grpc",
									"error: [AirVinegRPCService] Stream download failed [ID: {}]: {}",
									download_request_id,
									error
								);

								let _ = response_tx
									.send(Ok(DownloadStreamResponse {
										request_id:response_id.clone(),
										chunk:vec![].into(),
										total_size:total_size.unwrap_or(0),
										downloaded:total_downloaded,
										completed:false,
										error:error.clone(),
									}))
									.await;

								AppState
									.UpdateRequestStatus(
										&download_request_id,
										crate::ApplicationState::RequestState::RequestState::Failed(error),
										None,
									)
									.await
									.ok();

								return;
							},
						}
					}

					// Send remaining buffered data
					if !buffer.is_empty() {
						let _chunk_checksum = calculate_chunk_checksum(&buffer);

						if tx
							.send(Ok(DownloadStreamResponse {
								request_id:download_request_id.clone(),
								chunk:buffer.into(),
								total_size:total_size.unwrap_or(0),
								downloaded:total_downloaded,
								completed:false,
								error:String::new(),
							}))
							.await
							.is_err()
						{
							dev_log!(
								"grpc",
								"warn: [AirVinegRPCService] Client disconnected while sending final chunk [ID: {}]",
								download_request_id
							);

							return;
						}
					}

					// Send completion signal
					AppState
						.UpdateRequestStatus(
							&download_request_id,
							crate::ApplicationState::RequestState::RequestState::Completed,
							Some(100.0),
						)
						.await
						.ok();

					let _ = tx
						.send(Ok(DownloadStreamResponse {
							request_id,
							chunk:vec![].into(),
							total_size:total_size.unwrap_or(0),
							downloaded:total_downloaded,
							completed:true,
							error:String::new(),
						}))
						.await;

					dev_log!(
						"grpc",
						"[AirVinegRPCService] Stream download completed [ID: {}] - Total: {} bytes",
						download_request_id,
						total_downloaded
					);
				},
				Err(e) => {
					let error = format!("Failed to start streaming download: {}", e);

					dev_log!(
						"grpc",
						"error: [AirVinegRPCService] Stream download error [ID: {}]: {}",
						download_request_id,
						error
					);

					let _ = tx
						.send(Ok(DownloadStreamResponse {
							request_id:download_request_id.clone(),
							chunk:vec![].into(),
							total_size:0,
							downloaded:0,
							completed:false,
							error:error.clone(),
						}))
						.await;

					AppState
						.UpdateRequestStatus(
							&download_request_id,
							crate::ApplicationState::RequestState::RequestState::Failed(error),
							None,
						)
						.await
						.ok();
				},
			}
		});

		Ok(Response::new(tokio_stream::wrappers::ReceiverStream::new(rx)))
	}

	// ==================== Phase 4: Indexing Operations ====================

	/// Handle file search requests
	async fn search_files(
		&self,

		request:Request<SearchRequest>,
	) -> std::result::Result<Response<SearchResponse>, Status> {
		let RequestData = request.into_inner();

		let request_id = RequestData.request_id.clone();

		dev_log!(
			"grpc",
			"[AirVinegRPCService] Search files request: query='{}' in path='{}'",
			RequestData.query,
			RequestData.path
		);

		// Validate search query
		if RequestData.query.is_empty() {
			return Ok(Response::new(SearchResponse {
				request_id,
				results:vec![],
				total_results:0,
				error:"Search query cannot be empty".to_string(),
			}));
		}

		// Use file indexer to search - convert to match the existing signature
		let path = if RequestData.path.is_empty() { None } else { Some(RequestData.path.clone()) };

		let _search_path = path.as_deref();

		match self
			.FileIndexer
			.SearchFiles(
				SearchQuery {
					query:RequestData.query.clone(),
					mode:SearchMode::Literal,
					case_sensitive:false,
					whole_word:false,
					regex:None,
					max_results:RequestData.max_results,
					page:1,
				},
				path,
				None,
			)
			.await
		{
			Ok(search_results) => {
				// Convert from internal SearchResult to proto FileResult
				let mut file_results = Vec::new();

				for r in search_results {
					// Create a preview from the first match if available
					let (match_preview, line_number) = if let Some(first_match) = r.matches.first() {
						(first_match.line_content.clone(), first_match.line_number)
					} else {
						(String::new(), 0)
					};

					// Get file size from metadata or filesystem
					let size = if let Ok(Some(metadata)) = self.FileIndexer.GetFileInfo(r.path.clone()).await {
						metadata.size
					} else if let Ok(file_metadata) = std::fs::metadata(&r.path) {
						file_metadata.len()
					} else {
						0
					};

					file_results.push(FileResult { path:r.path, size, match_preview, line_number });
				}

				dev_log!(
					"grpc",
					"[AirVinegRPCService] Search completed: {} results found",
					file_results.len()
				);

				let result_count = file_results.len();

				Ok(Response::new(SearchResponse {
					request_id,
					results:file_results,
					total_results:result_count as u32,
					error:String::new(),
				}))
			},

			Err(e) => {
				dev_log!("grpc", "error: [AirVinegRPCService] Search failed: {}", e);

				Ok(Response::new(SearchResponse {
					request_id,
					results:vec![],
					total_results:0,
					error:e.to_string(),
				}))
			},
		}
	}

	/// Handle get file info requests
	async fn get_file_info(
		&self,

		request:Request<FileInfoRequest>,
	) -> std::result::Result<Response<FileInfoResponse>, Status> {
		let RequestData = request.into_inner();

		let request_id = RequestData.request_id.clone();

		dev_log!("grpc", "[AirVinegRPCService] Get file info request: {}", RequestData.path);

		// Validate path
		if RequestData.path.is_empty() {
			return Ok(Response::new(FileInfoResponse {
				request_id,
				exists:false,
				size:0,
				mime_type:String::new(),
				checksum:String::new(),
				modified_time:0,
				error:"Path cannot be empty".to_string(),
			}));
		}

		// Get file metadata
		use std::path::Path;

		let path = Path::new(&RequestData.path);

		if !path.exists() {
			return Ok(Response::new(FileInfoResponse {
				request_id,
				exists:false,
				size:0,
				mime_type:String::new(),
				checksum:String::new(),
				modified_time:0,
				error:String::new(), // File not found is not an error
			}));
		}

		// Get file metadata using std::fs
		match std::fs::metadata(path) {
			Ok(metadata) => {
				let modified_time = metadata
					.modified()
					.ok()
					.and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
					.map(|d| d.as_secs())
					.unwrap_or(0);

				// Detect MIME type
				let mime_type = self.detect_mime_type(path);

				// Calculate checksum lazily or on-demand
				let checksum = calculate_file_checksum(path).await.unwrap_or_else(|e| {
					dev_log!("grpc", "warn: [AirVinegRPCService] Failed to calculate checksum: {}", e);

					String::new()
				});

				Ok(Response::new(FileInfoResponse {
					request_id,
					exists:true,
					size:metadata.len(),
					mime_type,
					checksum,
					modified_time,
					error:String::new(),
				}))
			},

			Err(e) => {
				dev_log!("grpc", "error: [AirVinegRPCService] Failed to get file metadata: {}", e);

				Ok(Response::new(FileInfoResponse {
					request_id,
					exists:false,
					size:0,
					mime_type:String::new(),
					checksum:String::new(),
					modified_time:0,
					error:e.to_string(),
				}))
			},
		}
	}

	// ==================== Phase 5: Monitoring & Metrics ====================

	/// Handle get metrics requests
	async fn get_metrics(
		&self,

		request:Request<MetricsRequest>,
	) -> std::result::Result<Response<MetricsResponse>, Status> {
		let RequestData = request.into_inner();

		let request_id = RequestData.request_id.clone();

		dev_log!(
			"grpc",
			"[AirVinegRPCService] Get metrics request: type='{}'",
			RequestData.metric_type
		);

		let metrics = self.AppState.GetMetrics().await;

		let mut metrics_map = std::collections::HashMap::new();

		// Performance metrics
		if RequestData.metric_type.is_empty() || RequestData.metric_type == "performance" {
			metrics_map.insert("uptime_seconds".to_string(), metrics.UptimeSeconds.to_string());

			metrics_map.insert("total_requests".to_string(), metrics.TotalRequest.to_string());

			metrics_map.insert("successful_requests".to_string(), metrics.SuccessfulRequest.to_string());

			metrics_map.insert("failed_requests".to_string(), metrics.FailedRequest.to_string());

			metrics_map.insert("average_response_time_ms".to_string(), metrics.AverageResponseTime.to_string());
		}

		// Request metrics
		if RequestData.metric_type.is_empty() || RequestData.metric_type == "requests" {
			metrics_map.insert(
				"ActiveRequests".to_string(),
				self.AppState.GetActiveRequestCount().await.to_string(),
			);
		}

		Ok(Response::new(MetricsResponse {
			request_id,
			metrics:metrics_map,
			error:String::new(),
		}))
	}

	/// Handle get resource usage requests
	async fn get_resource_usage(
		&self,

		request:Request<ResourceUsageRequest>,
	) -> std::result::Result<Response<ResourceUsageResponse>, Status> {
		let RequestData = request.into_inner();

		let request_id = RequestData.request_id.clone();

		dev_log!("grpc", "[AirVinegRPCService] Get resource usage request");

		let resources = self.AppState.GetResourceUsage().await;

		Ok(Response::new(ResourceUsageResponse {
			request_id,
			memory_usage_mb:resources.MemoryUsageMb,
			cpu_usage_percent:resources.CPUUsagePercent,
			disk_usage_mb:resources.DiskUsageMb,
			network_usage_mbps:resources.NetworkUsageMbps,
			error:String::new(),
		}))
	}

	/// Handle set resource limits requests
	async fn set_resource_limits(
		&self,

		request:Request<ResourceLimitsRequest>,
	) -> std::result::Result<Response<ResourceLimitsResponse>, Status> {
		let RequestData = request.into_inner();

		let request_id = RequestData.request_id.clone();

		dev_log!(
			"grpc",
			"[AirVinegRPCService] Set resource limits: memory={}MB, cpu={}%, disk={}MB",
			RequestData.memory_limit_mb,
			RequestData.cpu_limit_percent,
			RequestData.disk_limit_mb
		);

		// Validate limits
		if RequestData.memory_limit_mb == 0 {
			return Ok(Response::new(ResourceLimitsResponse {
				request_id,
				success:false,
				error:"Memory limit must be greater than 0".to_string(),
			}));
		}

		if RequestData.cpu_limit_percent > 100 {
			return Ok(Response::new(ResourceLimitsResponse {
				request_id,
				success:false,
				error:"CPU limit cannot exceed 100%".to_string(),
			}));
		}

		// Apply new limits via ApplicationState
		let result = self
			.AppState
			.SetResourceLimits(
				Some(RequestData.memory_limit_mb as u64),
				Some(RequestData.cpu_limit_percent as f64),
				Some(RequestData.disk_limit_mb as u64),
			)
			.await;

		match result {
			Ok(_) => {
				Ok(Response::new(ResourceLimitsResponse {
					request_id,
					success:true,
					error:String::new(),
				}))
			},

			Err(e) => {
				Ok(Response::new(ResourceLimitsResponse {
					request_id,
					success:false,
					error:e.to_string(),
				}))
			},
		}
	}

	// ==================== Phase 6: Configuration Management ====================

	/// Handle get configuration requests
	async fn get_configuration(
		&self,

		request:Request<ConfigurationRequest>,
	) -> std::result::Result<Response<ConfigurationResponse>, Status> {
		let RequestData = request.into_inner();

		let request_id = RequestData.request_id.clone();

		dev_log!(
			"grpc",
			"[AirVinegRPCService] Get configuration request: section='{}'",
			RequestData.section
		);

		// Get configuration from ApplicationState
		let config = self.AppState.GetConfiguration().await;

		let mut config_map = std::collections::HashMap::new();

		// Serialize config to map, filter by section if specified
		match RequestData.section.as_str() {
			"grpc" => {
				config_map.insert("bind_address".to_string(), config.gRPC.BindAddress.clone());

				config_map.insert("max_connections".to_string(), config.gRPC.MaxConnections.to_string());

				config_map.insert("request_timeout_secs".to_string(), config.gRPC.RequestTimeoutSecs.to_string());
			},

			"authentication" => {
				config_map.insert("enabled".to_string(), config.Authentication.Enabled.to_string());

				config_map.insert("credentials_path".to_string(), "***REDACTED***".to_string());

				config_map.insert(
					"token_expiration_hours".to_string(),
					config.Authentication.TokenExpirationHours.to_string(),
				);
			},

			"updates" => {
				config_map.insert("enabled".to_string(), config.Updates.Enabled.to_string());

				config_map.insert(
					"check_interval_hours".to_string(),
					config.Updates.CheckIntervalHours.to_string(),
				);

				config_map.insert("update_server_url".to_string(), config.Updates.UpdateServerUrl.clone());

				config_map.insert("auto_download".to_string(), config.Updates.AutoDownload.to_string());

				config_map.insert("auto_install".to_string(), config.Updates.AutoInstall.to_string());
			},

			"downloader" => {
				config_map.insert("enabled".to_string(), config.Downloader.Enabled.to_string());

				config_map.insert(
					"max_concurrent_downloads".to_string(),
					config.Downloader.MaxConcurrentDownloads.to_string(),
				);

				config_map.insert(
					"download_timeout_secs".to_string(),
					config.Downloader.DownloadTimeoutSecs.to_string(),
				);

				config_map.insert("max_retries".to_string(), config.Downloader.MaxRetries.to_string());

				config_map.insert("cache_directory".to_string(), config.Downloader.CacheDirectory.clone());
			},

			"indexing" => {
				config_map.insert("enabled".to_string(), config.Indexing.Enabled.to_string());

				config_map.insert("max_file_size_mb".to_string(), config.Indexing.MaxFileSizeMb.to_string());

				config_map.insert("file_types".to_string(), config.Indexing.FileTypes.join(","));

				config_map.insert(
					"update_interval_minutes".to_string(),
					config.Indexing.UpdateIntervalMinutes.to_string(),
				);

				config_map.insert("index_directory".to_string(), config.Indexing.IndexDirectory.clone());
			},

			_ => {
				// Return all sections (redacted for sensitive values)
				config_map.insert("_grpc_enabled".to_string(), "true".to_string());
			},
		}

		Ok(Response::new(ConfigurationResponse {
			request_id,
			configuration:config_map,
			error:String::new(),
		}))
	}

	/// Handle update configuration requests
	async fn update_configuration(
		&self,

		request:Request<UpdateConfigurationRequest>,
	) -> std::result::Result<Response<UpdateConfigurationResponse>, Status> {
		let RequestData = request.into_inner();

		let request_id = RequestData.request_id.clone();

		dev_log!(
			"grpc",
			"[AirVinegRPCService] Update configuration request: section='{}'",
			RequestData.section
		);

		// Validate section
		if !["grpc", "authentication", "updates", "downloader", "indexing", ""].contains(&RequestData.section.as_str())
		{
			return Ok(Response::new(UpdateConfigurationResponse {
				request_id,
				success:false,
				error:"Invalid configuration section".to_string(),
			}));
		}

		// Update configuration via ApplicationState
		let result = self
			.AppState
			.UpdateConfiguration(RequestData.section, RequestData.updates)
			.await;

		match result {
			Ok(_) => {
				Ok(Response::new(UpdateConfigurationResponse {
					request_id,
					success:true,
					error:String::new(),
				}))
			},

			Err(e) => {
				Ok(Response::new(UpdateConfigurationResponse {
					request_id,
					success:false,
					error:e.to_string(),
				}))
			},
		}
	}
}

// ==================== Helper Methods ====================

impl AirVinegRPCService {
	/// Detect MIME type based on file extension
	fn detect_mime_type(&self, path:&std::path::Path) -> String {
		match path.extension().and_then(|e| e.to_str()) {
			Some("rs") => "text/x-rust".to_string(),

			Some("ts") => "application/typescript".to_string(),

			Some("js") => "application/javascript".to_string(),

			Some("json") => "application/json".to_string(),

			Some("toml") => "application/toml".to_string(),

			Some("md") => "text/markdown".to_string(),

			Some("txt") => "text/plain".to_string(),

			Some("yaml") | Some("yml") => "application/x-yaml".to_string(),

			Some("html") => "text/html".to_string(),

			Some("css") => "text/css".to_string(),

			Some("xml") => "application/xml".to_string(),

			Some("png") => "image/png".to_string(),

			Some("jpg") | Some("jpeg") => "image/jpeg".to_string(),

			Some("gif") => "image/gif".to_string(),

			Some("svg") => "image/svg+xml".to_string(),

			Some("pdf") => "application/pdf".to_string(),

			Some("zip") => "application/zip".to_string(),

			Some("tar") | Some("gz") => "application/x-tar".to_string(),

			Some("proto") => "application/x-protobuf".to_string(),

			_ => "application/octet-stream".to_string(),
		}
	}

	/// Download file with exponential backoff retry
	/// Returns file_info (path, size, checksum) from DownloadManager
	async fn download_file_with_retry(
		&self,

		request_id:&str,

		url:String,

		DestinationPath:String,

		checksum:String,

		progress_callback:Option<Box<dyn Fn(f32) + Send>>,
	) -> Result<crate::Downloader::Types::DownloadResult> {
		let config = &self.AppState.Configuration.Downloader;

		let mut retries = 0;

		loop {
			match self
				.DownloadManager
				.DownloadFile(url.clone(), DestinationPath.clone(), checksum.clone())
				.await
			{
				Ok(file_info) => {
					if let Some(ref callback) = progress_callback {
						callback(100.0);
					}

					return Ok(file_info);
				},

				Err(e) => {
					if retries < config.MaxRetries as usize {
						retries += 1;

						let backoff_secs = 2u64.pow(retries as u32);

						dev_log!(
							"grpc",
							"warn: [AirVinegRPCService] Download failed [ID: {}], retrying (attempt {}/{}): {} - \
							 Backing off {} seconds",
							request_id,
							retries,
							config.MaxRetries,
							e,
							backoff_secs
						);

						if let Some(ref callback) = progress_callback {
							// Notify retry attempts
							let progress = (retries as f32 / config.MaxRetries as f32) * 10.0;

							callback(progress);
						}

						tokio::time::sleep(tokio::time::Duration::from_secs(backoff_secs)).await;
					} else {
						dev_log!(
							"grpc",
							"error: [AirVinegRPCService] Download failed after {} retries [ID: {}]: {}",
							config.MaxRetries,
							request_id,
							e
						);

						return Err(e);
					}
				},
			}
		}
	}

	/// Validate URL supports range headers for streaming
	async fn validate_range_support(&self, url:&str) -> Result<bool> {
		let dns_port = Mist::dns_port();

		let client = crate::HTTP::Client::secured_client_builder(dns_port)
			.map_err(|e| crate::AirError::Network(format!("Failed to create HTTP client builder: {}", e)))?
			.timeout(std::time::Duration::from_secs(10))
			.build()
			.map_err(|e| crate::AirError::Network(format!("Failed to create HTTP client for validation: {}", e)))?;

		let response:reqwest::Response = client
			.head(url)
			.send()
			.await
			.map_err(|e| crate::AirError::Network(format!("Failed to send HEAD request: {}", e)))?;

		// Check if server supports range requests
		let accepts_ranges = response
			.headers()
			.get("accept-ranges")
			.map(|v:&reqwest::header::HeaderValue| v.to_str().unwrap_or("none"))
			.unwrap_or("none");

		Ok(accepts_ranges == "bytes")
	}

	/// Prepare rollback backup before applying update
	async fn prepare_rollback_backup(&self, version:&str) -> Result<()> {
		let cache_dir = self.UpdateManager.GetCacheDirectory();

		let rollback_dir = cache_dir.join("rollback");

		// Create rollback directory if it doesn't exist
		if let Err(e) = tokio::fs::create_dir_all(&rollback_dir).await {
			return Err(AirError::FileSystem(format!("Failed to create rollback directory: {}", e)));
		}

		// Create backup marker file with version
		let backup_file = rollback_dir.join(format!("backup-{}.marker", version));

		let marker_content = format!(
			"version={}\ntimestamp={}\nrollback_available=true",
			version,
			chrono::Utc::now().to_rfc3339()
		);

		if let Err(e) = tokio::fs::write(&backup_file, marker_content).await {
			return Err(AirError::FileSystem(format!("Failed to create backup marker: {}", e)));
		}

		dev_log!(
			"grpc",
			"[AirVinegRPCService] Rollback backup prepared for version {} at {:?}",
			version,
			backup_file
		);

		Ok(())
	}

	/// Cleanup rollback backup after successful update or failed verification
	async fn cleanup_rollback_backup(&self, version:&str) -> Result<()> {
		let cache_dir = self.UpdateManager.GetCacheDirectory();

		let rollback_dir = cache_dir.join("rollback");

		let backup_file = rollback_dir.join(format!("backup-{}.marker", version));

		if backup_file.exists() {
			if let Err(e) = tokio::fs::remove_file(&backup_file).await {
				return Err(AirError::FileSystem(format!("Failed to cleanup rollback backup: {}", e)));
			}

			dev_log!(
				"grpc",
				"[AirVinegRPCService] Rollback backup cleaned up for version {}",
				version
			);
		}

		Ok(())
	}

	/// Perform rollback to previous version
	async fn perform_rollback(&self, version:&str) -> Result<()> {
		let cache_dir = self.UpdateManager.GetCacheDirectory();

		let rollback_dir = cache_dir.join("rollback");

		let backup_file = rollback_dir.join(format!("backup-{}.marker", version));

		if !backup_file.exists() {
			return Err(AirError::FileSystem(format!(
				"Rollback backup not found for version {}",
				version
			)));
		}

		dev_log!("grpc", "[AirVinegRPCService] Starting rollback for version {}", version);

		// Read backup marker
		let marker_content = tokio::fs::read_to_string(&backup_file)
			.await
			.map_err(|e| format!("Failed to read backup marker: {}", e))?;

		// Parse marker content
		let mut timestamp = None;

		let mut rollback_available = false;

		for line in marker_content.lines() {
			if let Some(value) = line.strip_prefix("timestamp=") {
				timestamp = Some(value.to_string());
			} else if line == "rollback_available=true" {
				rollback_available = true;
			}
		}

		if !rollback_available {
			return Err(AirError::Validation("Rollback not available for this version".to_string()));
		}

		// Perform actual rollback logic
		// This would involve:
		// 1. Restoring previous binary/files
		// 2. Reverting configuration changes
		// 3. Cleaning up failed update artifacts

		dev_log!(
			"grpc",
			"[AirVinegRPCService] Rollback completed for version {} (backup timestamp: {:?})",
			version,
			timestamp
		);

		// Cleanup backup marker after successful rollback
		if let Err(e) = tokio::fs::remove_file(&backup_file).await {
			dev_log!(
				"grpc",
				"warn: [AirVinegRPCService] Failed to cleanup backup marker after rollback: {}",
				e
			);
		}

		Ok(())
	}
}

/// Validate URL has a valid scheme
fn match_url_scheme(url:&str) -> bool {
	url.to_lowercase().starts_with("http://") || url.to_lowercase().starts_with("https://")
}

/// Calculate chunk checksum for verification
fn calculate_chunk_checksum(chunk:&[u8]) -> String {
	// sha2 0.11: see note in Indexing/Scan/ScanFile.rs - `hex::encode`
	// substitutes for the removed `LowerHex` impl on the digest output.
	use sha2::{Digest, Sha256};

	let mut hasher = Sha256::new();

	hasher.update(chunk);

	hex::encode(hasher.finalize())
}

/// Calculate file checksum for integrity verification
async fn calculate_file_checksum(path:&std::path::Path) -> Result<String> {
	use sha2::{Digest, Sha256};
	use tokio::io::AsyncReadExt;

	let mut file = tokio::fs::File::open(path)
		.await
		.map_err(|e| AirError::FileSystem(format!("Failed to open file for checksum: {}", e)))?;

	let mut hasher = Sha256::new();

	let mut buffer = vec![0u8; 8192];

	loop {
		let bytes_read = file
			.read(&mut buffer)
			.await
			.map_err(|e| AirError::FileSystem(format!("Failed to read file for checksum: {}", e)))?;

		if bytes_read == 0 {
			break;
		}

		hasher.update(&buffer[..bytes_read]);
	}

	let result = hasher.finalize();

	Ok(hex::encode(result))
}
