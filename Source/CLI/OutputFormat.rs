//! Enum representing the format in which CLI output is rendered.

/// Output format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
	Plain,

	Table,

	Json,
}
