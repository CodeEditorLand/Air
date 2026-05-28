//! Information about file indexing.

#[derive(Debug, Clone)]
pub struct Struct {
	pub files_indexed:u32,

	pub total_size:u64,
}
