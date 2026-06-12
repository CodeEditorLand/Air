use serde::{Deserialize, Serialize};

use crate::ApplicationState::ConnectionType::ConnectionType;

/// Connection information for Mountain clients
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionInfo {
	pub ConnectionId:String,

	pub ClientId:String,

	pub ClientVersion:String,

	pub ProtocolVersion:u32,

	pub LastHeartbeat:u64,

	pub IsActive:bool,

	pub ConnectionType:ConnectionType,
}
