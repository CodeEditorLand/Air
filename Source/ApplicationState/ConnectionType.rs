use serde::{Deserialize, Serialize};

/// Connection type enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConnectionType {
	MountainMain,

	MountainWorker,

	Cocoon,

	Wind,

	External,
}
