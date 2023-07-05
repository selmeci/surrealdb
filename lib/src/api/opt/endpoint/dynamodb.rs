use crate::api::engine::local::Db;
use crate::api::engine::local::DynamoDb;
use crate::api::err::Error;
use crate::api::opt::Endpoint;
use crate::api::opt::IntoEndpoint;
use crate::api::opt::Strict;
use crate::api::Result;
use crate::dbs::Level;
use crate::opt::auth::Root;
use std::fmt::Display;
use url::Url;

impl IntoEndpoint<DynamoDb> for &str {
	type Client = Db;

	fn into_endpoint(self) -> Result<Endpoint> {
		let url = self.to_string();
		Ok(Endpoint {
			endpoint: Url::parse(&url).map_err(|_| Error::InvalidUrl(url))?,
			strict: false,
			#[cfg(any(feature = "native-tls", feature = "rustls"))]
			tls_config: None,
			auth: Level::No,
			username: String::new(),
			password: String::new(),
		})
	}
}

impl IntoEndpoint<DynamoDb> for String {
	type Client = Db;

	fn into_endpoint(self) -> Result<Endpoint> {
		let url = self.to_string();
		Ok(Endpoint {
			endpoint: Url::parse(&url).map_err(|_| Error::InvalidUrl(url))?,
			strict: false,
			#[cfg(any(feature = "native-tls", feature = "rustls"))]
			tls_config: None,
			auth: Level::No,
			username: String::new(),
			password: String::new(),
		})
	}
}

impl<T> IntoEndpoint<DynamoDb> for (T, Strict)
where
	T: IntoEndpoint<DynamoDb> + Display,
{
	type Client = Db;

	fn into_endpoint(self) -> Result<Endpoint> {
		let (address, _) = self;
		let mut endpoint = address.into_endpoint()?;
		endpoint.strict = true;
		Ok(endpoint)
	}
}

impl<T> IntoEndpoint<DynamoDb> for (T, Root<'_>)
where
	T: IntoEndpoint<DynamoDb> + Display,
{
	type Client = Db;

	fn into_endpoint(self) -> Result<Endpoint> {
		let (address, root) = self;
		let mut endpoint = address.into_endpoint()?;
		endpoint.auth = Level::Kv;
		endpoint.username = root.username.to_owned();
		endpoint.password = root.password.to_owned();
		Ok(endpoint)
	}
}

impl<T> IntoEndpoint<DynamoDb> for (T, Strict, Root<'_>)
where
	T: IntoEndpoint<DynamoDb> + Display,
{
	type Client = Db;

	fn into_endpoint(self) -> Result<Endpoint> {
		let (address, _, root) = self;
		let mut endpoint = (address, root).into_endpoint()?;
		endpoint.strict = true;
		Ok(endpoint)
	}
}
