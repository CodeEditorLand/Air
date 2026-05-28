//! Information about a downloaded file.

#[derive(Debug, Clone)]
pub struct Struct {
	pub file_path:String,

	pub file_size:u64,

	pub checksum:String,
}
