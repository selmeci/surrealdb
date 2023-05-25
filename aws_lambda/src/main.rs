use lambda_web::{run_hyper_on_lambda, LambdaError};
use std::env;
use surreal::{init_warp, LambdaConfig};

#[tokio::main]
async fn main() -> Result<(), LambdaError> {
	let table = env::var("TABLE").expect("Missing DynamoDB table name. $TABLE");
	let stage = env::var("STAGE").expect("API GTW stage is missing. $STAGE");
	let strict = env::var("STRICT").map_or(false, |v| v.eq("true"));
	let user = env::var("USER").unwrap_or("root".to_string());
	let log = env::var("LOG_LVL").unwrap_or("info".to_string());
	let pass = env::var("PASS").ok();

	let routes = init_warp(LambdaConfig {
		strict,
		user,
		pass,
		table,
		stage,
		log,
	})
	.await
	.expect("SurrealDB is not working!");

	let warp_service = warp::service(routes);
	run_hyper_on_lambda(warp_service).await?;
	Ok(())
}
