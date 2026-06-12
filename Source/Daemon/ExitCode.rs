/// Exit codes for daemon operations
#[derive(Debug, Clone)]
pub enum ExitCode {
	Success = 0,

	ConfigurationError = 1,

	AlreadyRunning = 2,

	PermissionDenied = 3,

	ServiceError = 4,

	ResourceError = 5,

	NetworkError = 6,

	AuthenticationError = 7,

	FileSystemError = 8,

	InternalError = 9,

	UnknownError = 10,
}

impl From<ExitCode> for i32 {
	fn from(code:ExitCode) -> i32 { code as i32 }
}
