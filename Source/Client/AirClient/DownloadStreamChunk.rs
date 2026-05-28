//! Single chunk of data from a streaming download. Carries the binary
//! payload plus progress + completion metadata.

#[derive(Debug, Clone)]
pub struct Struct {
	pub data:Vec<u8>,

	pub total_size:u64,

	pub downloaded:u64,

	pub completed:bool,

	pub error:String,
}
