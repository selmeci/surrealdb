use once_cell::sync::OnceCell;
use std::net::SocketAddr;

pub static CF: OnceCell<Config> = OnceCell::new();

#[derive(Clone, Debug)]
pub struct Config {
	pub strict: bool,
	pub bind: SocketAddr,
	pub path: String,
	pub user: String,
	pub pass: Option<String>,
	pub crt: Option<String>,
	pub key: Option<String>,
}

#[cfg(feature = "aws-lambda")]
pub struct LambdaConfig {
	pub strict: bool,
	pub table: String,
	pub user: String,
	pub pass: Option<String>,
	pub log: String,
}

#[cfg(not(feature = "aws-lambda"))]
pub fn init(matches: &clap::ArgMatches) {
	// Parse the server binding address
	let bind = matches.value_of("bind").unwrap().parse::<SocketAddr>().unwrap();
	// Parse the database endpoint path
	let path = matches.value_of("path").unwrap().to_owned();
	// Parse the root username for authentication
	let user = matches.value_of("user").unwrap().to_owned();
	// Parse the root password for authentication
	let pass = matches.value_of("pass").map(|v| v.to_owned());
	// Parse any TLS server security options
	let crt = matches.value_of("web-crt").map(|v| v.to_owned());
	let key = matches.value_of("web-key").map(|v| v.to_owned());
	// Check if database strict mode is enabled
	let strict = matches.is_present("strict");
	// Store the new config object
	let _ = CF.set(Config {
		strict,
		bind,
		path,
		user,
		pass,
		crt,
		key,
	});
}

#[cfg(feature = "aws-lambda")]
pub fn init(config: &LambdaConfig) {
	// Store the new config object
	let _ = CF.set(Config {
		strict: config.strict,
		bind: "127.0.0.1:80".parse::<SocketAddr>().unwrap(), // ignored for AWS Lambda
		path: format!("dynamodb://{}", config.table),
		user: config.user.to_owned(),
		pass: config.pass.to_owned(),
		crt: None,
		key: None,
	});
}
