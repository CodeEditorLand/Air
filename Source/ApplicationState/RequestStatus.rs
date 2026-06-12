use crate::ApplicationState::RequestState::RequestState;

/// Request status tracking
#[derive(Debug, Clone)]
pub struct RequestStatus {
	pub RequestId:String,

	pub Service:String,

	pub StartedAt:u64,

	pub Status:RequestState,

	pub Progress:Option<f32>,
}
