//! Information about an available update.

#[derive(Debug, Clone)]
pub struct Struct {
	pub update_available:bool,

	pub version:String,

	pub download_url:String,

	pub release_notes:String,
}
