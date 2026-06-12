#[cfg(test)]
mod tests {

	use super::super::CommandTypes::{Command, ConfigCommand};
	use super::super::CliParser::CliParser;

	#[test]
	fn test_parse_status_command() {
		let args = vec!["Air".to_string(), "status".to_string(), "--verbose".to_string()];

		let cmd = CliParser::parse(args).unwrap();

		if let Command::Status { service, verbose, json } = cmd {
			assert!(verbose);

			assert!(!json);

			assert!(service.is_none());
		} else {
			panic!("Expected Status command");
		}
	}

	#[test]
	fn test_parse_config_set() {
		let args = vec![
			"Air".to_string(),
			"config".to_string(),
			"set".to_string(),
			"grpc.bind_address".to_string(),
			"[::1]:50053".to_string(),
		];

		let cmd = CliParser::parse(args).unwrap();

		if let Command::Config(ConfigCommand::Set { key, value }) = cmd {
			assert_eq!(key, "grpc.bind_address");

			assert_eq!(value, "[::1]:50053");
		} else {
			panic!("Expected Config Set command");
		}
	}
}
