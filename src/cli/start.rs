use super::config;
#[cfg(feature = "aws-lambda")]
use crate::cli::config::LambdaConfig;
use crate::cnf::LOGO;
use crate::dbs;
use crate::env;
use crate::err::Error;
use crate::iam;
use crate::net;
use futures::Future;

#[cfg(feature = "aws-lambda")]
type InitMatches = LambdaConfig;
#[cfg(not(feature = "aws-lambda"))]
type InitMatches = clap::ArgMatches;

#[cfg(not(feature = "aws-lambda"))]
pub fn init(matches: &InitMatches) -> Result<(), Error> {
	with_enough_stack(init_impl(matches))
}

#[cfg(feature = "aws-lambda")]
pub async fn init(matches: &InitMatches) -> Result<(), Error> {
	init_impl(matches).await
}

async fn init_impl(matches: &InitMatches) -> Result<(), Error> {
	// Initialize opentelemetry and logging
	#[cfg(not(feature = "aws-lambda"))]
	crate::o11y::builder().with_log_level(matches.get_one::<String>("log").unwrap()).init();
	#[cfg(feature = "aws-lambda")]
	crate::o11y::builder().with_log_level(&matches.log).init();
	// Check if a banner should be outputted
	#[cfg(not(feature = "aws-lambda"))]
	if !matches.is_present("no-banner") {
		// Output SurrealDB logo
		println!("{LOGO}");
	}
	// Setup the cli options
	config::init(matches);
	// Initiate environment
	env::init().await?;
	// Initiate master auth
	iam::init().await?;
	// Start the kvs server
	dbs::init().await?;
	// Start the web server
	#[cfg(not(feature = "aws-lambda"))]
	net::init().await?;
	// All ok
	Ok(())
}

/// Rust's default thread stack size of 2MiB doesn't allow sufficient recursion depth.
fn with_enough_stack<T>(fut: impl Future<Output = T> + Send) -> T {
	let stack_size = 8 * 1024 * 1024;

	// Stack frames are generally larger in debug mode.
	#[cfg(debug_assertions)]
	let stack_size = stack_size * 2;

	tokio::runtime::Builder::new_multi_thread()
		.enable_all()
		.thread_stack_size(stack_size)
		.build()
		.unwrap()
		.block_on(fut)
}
