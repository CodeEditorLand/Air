/// Request state enum
#[derive(Debug, Clone)]
pub enum RequestState {
	Pending,

	InProgress,

	Completed,

	Failed(String),

	Cancelled,
}
