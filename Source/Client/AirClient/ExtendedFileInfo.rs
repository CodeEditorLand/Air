//! Extended file information.

#[derive(Debug, Clone)]
pub struct Struct {
	pub exists:bool,

	pub size:u64,

	pub mime_type:String,

	pub checksum:String,

	pub modified_time:u64,
}
