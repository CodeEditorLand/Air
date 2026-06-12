use serde::{Deserialize, Serialize};

use super::{ResourceWarningType::ResourceWarningType, WarningSeverity::WarningSeverity};

/// Resource warning types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceWarning {
	pub WarningType:ResourceWarningType,

	pub ServiceName:Option<String>,

	pub CurrentValue:f64,

	pub Threshold:f64,

	pub Severity:WarningSeverity,

	pub Timestamp:u64,
}
