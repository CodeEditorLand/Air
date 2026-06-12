use std::collections::HashMap;

/// Indexing result with statistics
#[derive(Debug, Clone)]
pub struct IndexResult {
	/// Number of files successfully indexed
	pub files_indexed:u32,

	/// Total size of indexed files in bytes
	pub total_size:u64,

	/// Time taken in seconds
	pub duration_seconds:f64,

	/// Number of symbols extracted
	pub symbols_extracted:u32,

	/// Number of files with errors
	pub files_with_errors:u32,
}

/// Index statistics
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IndexStatistics {
	pub file_count:u32,

	pub total_size:u64,

	pub total_symbols:u32,

	pub language_counts:HashMap<String, u32>,

	pub last_updated:chrono::DateTime<chrono::Utc>,

	pub index_version:String,
}
