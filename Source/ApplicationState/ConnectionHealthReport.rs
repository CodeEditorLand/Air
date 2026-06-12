use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Connection health report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionHealthReport {
	pub TotalConnection:usize,

	pub HealthyConnection:usize,

	pub StaleConnection:usize,

	pub ConnectionByType:HashMap<String, usize>,

	pub LastChecked:u64,
}
