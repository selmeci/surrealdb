use lambda_web::{run_hyper_on_lambda, LambdaError};
use std::env;
use surreal::{
	init_warp, ClientIp, CustomEnvFilter, EnvFilter, StartCommandArguments, StartCommandDbsOptions,
};

#[tokio::main]
async fn main() -> Result<(), LambdaError> {
	let table = env::var("TABLE").expect("Missing DynamoDB table name. $TABLE");
	let shards = env::var("SHARDS").unwrap_or("1".to_string());
	let strict = env::var("STRICT").map_or(false, |v| v.eq("true"));
	let username = env::var("USER").unwrap_or("root".to_string());
	let log = env::var("LOG_LVL").unwrap_or("info".to_string());
	let password = env::var("PASS").ok();

	let routes = init_warp(StartCommandArguments {
		path: format!("dynamodb://{}?shards={}", table, shards),
		username,
		password,
		allowed_networks: vec!["0.0.0.0/32".into()],
		client_ip: ClientIp::None,
		listen_addresses: vec!["0.0.0.0:80".into()],
		dbs: StartCommandDbsOptions {
			query_timeout: None,
		},
		key: None,
		kvs: None,
		web: None,
		strict,
		log: CustomEnvFilter(EnvFilter::builder().parse(format!(
			"error,surreal={log},surrealdb={log},surrealdb::txn=error",
			log = log
		))?),
		no_banner: true,
	})
	.await
	.expect("SurrealDB is not working!");

	let warp_service = warp::service(routes);
	run_hyper_on_lambda(warp_service).await?;
	Ok(())
}
