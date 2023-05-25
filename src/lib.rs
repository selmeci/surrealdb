//! This binary is the web-platform server for [SurrealDB](https://surrealdb.com) the
//! ultimate cloud database for tomorrow's applications. SurrealDB is a scalable,
//! distributed, collaborative, document-graph database for the realtime web.
//!
//! This binary can be used to start a database server instance using an embedded
//! in-memory datastore, or an embedded datastore persisted to disk. In addition, it
//! can be used in distributed mode by connecting to a distributed [TiKV](https://tikv.org)
//! key-value store or AWS DynamoDB without transactional support.

#![deny(clippy::mem_forget)]
#![allow(dead_code)]
#![allow(unused_imports)]
#![forbid(unsafe_code)]

use crate::cli::start;
#[cfg(feature = "aws-lambda")]
pub use crate::cli::LambdaConfig;
pub use crate::err::Error;
use crate::net::warp_routes;
use warp::Filter;

#[macro_use]
extern crate log;

#[macro_use]
mod mac;

mod cli;
mod cnf;
mod dbs;
mod env;
mod err;
mod iam;
mod net;
mod o11y;
mod rpc;

#[cfg(feature = "aws-lambda")]
pub async fn init_warp(
	config: LambdaConfig,
) -> Result<
	impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone + Sync + Send + 'static,
	Error,
> {
	start(&config).await?;
	let routes = warp_routes();
	Ok(routes)
}
