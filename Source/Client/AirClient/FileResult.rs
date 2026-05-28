//! Result of a file search.

#[derive(Debug, Clone)]
pub struct Struct {
	pub path:String,

	pub size:u64,

	pub match_preview:String,

	pub line_number:u32,
}
