#![cfg(feature = "kv-dynamodb")]

use crate::err::Error;
use crate::kvs::Key;
use crate::kvs::Val;
use aws_sdk_dynamodb::operation::delete_item::builders::DeleteItemFluentBuilder;
use aws_sdk_dynamodb::operation::delete_item::DeleteItemError;
use aws_sdk_dynamodb::operation::put_item::builders::PutItemFluentBuilder;
use aws_sdk_dynamodb::operation::put_item::PutItemError;
use aws_sdk_dynamodb::primitives::Blob;
use aws_sdk_dynamodb::types::{AttributeValue, KeysAndAttributes};
use aws_sdk_dynamodb::Client;
use rand::Rng;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::ops::Range;
use std::sync::Arc;

static DYNAMODB_ENDPOINT: &str = "DYNAMODB_ENDPOINT";

fn use_custom_dynamodb_endpoint() -> Option<String> {
	std::env::var(DYNAMODB_ENDPOINT).ok()
}

fn shard(shards: &u8) -> u8 {
	let mut rng = rand::thread_rng();
	rng.gen_range(0u8..*shards)
}

///
/// Datastore for DynamoDB key-value store
/// This Datastore does not support transactions
///
/// Requirements on table:
/// - Partition key is `Binary` with name `key`
///
/// Requirement os index:
/// - index name: `GSI1`
/// - Partition key is `Binary` with name `gsi1pk`
/// - Sort key is `Binary` with name `key`
///
pub struct Datastore {
	client: Arc<Client>,
	table: Arc<String>,
	shards: u8,
}

pub struct Transaction {
	// Is the transaction complete?
	ok: bool,
	// Is the transaction read+write?
	rw: bool,
	// client
	client: Arc<Client>,
	// table
	table: Arc<String>,
	// number of shards
	shards: u8,
}

impl Datastore {
	/// Open a new database from ENV
	pub async fn new(table: String, shards: u8) -> Result<Datastore, Error> {
		let config = aws_config::load_from_env().await;
		let mut builder = aws_sdk_dynamodb::config::Builder::from(&config);
		if let Some(custom_dynamodb_endpoint) = use_custom_dynamodb_endpoint() {
			builder = builder.endpoint_url(custom_dynamodb_endpoint);
		}
		let client = Arc::new(Client::from_conf(builder.build()));
		Ok(Datastore {
			client,
			table: Arc::new(table),
			shards,
		})
	}
	/// Start a new transaction
	pub async fn transaction(&self, write: bool, _: bool) -> Result<Transaction, Error> {
		Ok(Transaction {
			ok: false,
			rw: write,
			client: Arc::clone(&self.client),
			table: Arc::clone(&self.table),
			shards: self.shards,
		})
	}
}

impl Transaction {
	fn build_put_request<K, V>(&self, key: K, val: V) -> PutItemFluentBuilder
	where
		K: Into<Key>,
		V: Into<Val>,
	{
		let key = key.into();
		let shard = shard(&self.shards);
		let key = AttributeValue::B(Blob::new(key));
		self.client
			.put_item()
			.table_name(self.table.as_ref())
			.item("pk", key.clone())
			.item("sk", key.clone())
			.item("value", AttributeValue::B(Blob::new(val.into())))
			.item("gsi1pk", AttributeValue::B(Blob::new(vec![shard])))
			.item("gsi1sk", key)
	}

	fn build_delete_request<K>(&self, key: K) -> DeleteItemFluentBuilder
	where
		K: Into<Key>,
	{
		let key = key.into();
		let key = AttributeValue::B(Blob::new(key));
		self.client
			.delete_item()
			.table_name(self.table.as_ref())
			.key("pk", key.clone())
			.key("sk", key)
	}

	/// Check if closed
	pub fn closed(&self) -> bool {
		self.ok
	}

	/// Cancel a transaction
	pub async fn cancel(&mut self) -> Result<(), Error> {
		// Check to see if transaction is closed
		if self.ok {
			return Err(Error::TxFinished);
		}
		// Mark this transaction as done
		self.ok = true;
		// Continue
		Ok(())
	}

	/// Commit a transaction
	pub async fn commit(&mut self) -> Result<(), Error> {
		// Check to see if transaction is closed
		if self.ok {
			return Err(Error::TxFinished);
		}
		// Check to see if transaction is writable
		if !self.rw {
			return Err(Error::TxReadonly);
		}
		// Mark this transaction as done
		self.ok = true;

		// Continue
		Ok(())
	}

	/// Check if a key exists
	pub async fn exi<K>(&mut self, key: K) -> Result<bool, Error>
	where
		K: Into<Key>,
	{
		// Check to see if transaction is closed
		if self.ok {
			return Err(Error::TxFinished);
		}
		let key = key.into();
		let key = AttributeValue::B(Blob::new(key));
		// Check the key
		let res = self
			.client
			.get_item()
			.table_name(self.table.as_ref())
			.key("pk", key.clone())
			.key("sk", key)
			.attributes_to_get("pk")
			.send()
			.await
			.map_err(|err| Error::Ds(err.to_string()))?;

		// Return result
		Ok(res.item().is_some())
	}

	/// Fetch a key from the database
	pub async fn get<K>(&mut self, key: K) -> Result<Option<Val>, Error>
	where
		K: Into<Key>,
	{
		// Check to see if transaction is closed
		if self.ok {
			return Err(Error::TxFinished);
		}
		let key = key.into();
		let key = AttributeValue::B(Blob::new(key));
		// Get the key
		let res = self
			.client
			.get_item()
			.table_name(self.table.as_ref())
			.key("pk", key.clone())
			.key("sk", key)
			.send()
			.await
			.map_err(|err| Error::Ds(err.to_string()))?;

		// Return result
		let data = res.item.map(|mut data| {
			let val = data.remove("value").expect("Item must contains value attribute");
			if let AttributeValue::B(blob) = val {
				blob.into_inner()
			} else {
				unreachable!("Item must contains blob value attribute")
			}
		});
		Ok(data)
	}

	/// Insert or update a key in the database
	pub async fn set<K, V>(&mut self, key: K, val: V) -> Result<(), Error>
	where
		K: Into<Key>,
		V: Into<Val>,
	{
		// Check to see if transaction is closed
		if self.ok {
			return Err(Error::TxFinished);
		}
		// Check to see if transaction is writable
		if !self.rw {
			return Err(Error::TxReadonly);
		}
		// Set the key
		let request = self.build_put_request(key, val);
		request.send().await.map_err(|err| Error::Ds(err.to_string()))?;

		// Return result
		Ok(())
	}

	/// Insert a key if it doesn't exist in the database
	pub async fn put<K, V>(&mut self, key: K, val: V) -> Result<(), Error>
	where
		K: Into<Key>,
		V: Into<Val>,
	{
		// Check to see if transaction is closed
		if self.ok {
			return Err(Error::TxFinished);
		}
		// Check to see if transaction is writable
		if !self.rw {
			return Err(Error::TxReadonly);
		}
		// Set the key if not exists
		let request =
			self.build_put_request(key, val).condition_expression("attribute_not_exists(pk)");

		request.send().await.map_err(|err| {
			let err = err.into_service_error();
			if let PutItemError::ConditionalCheckFailedException(_) = err {
				Error::Tx("KeyAlreadyExists".into())
			} else {
				Error::Ds(err.to_string())
			}
		})?;

		// Return result
		Ok(())
	}

	/// Insert a key if it doesn't exist in the database
	pub async fn putc<K, V>(&mut self, key: K, val: V, chk: Option<V>) -> Result<(), Error>
	where
		K: Into<Key>,
		V: Into<Val>,
	{
		// Check to see if transaction is closed
		if self.ok {
			return Err(Error::TxFinished);
		}
		// Check to see if transaction is writable
		if !self.rw {
			return Err(Error::TxReadonly);
		}
		let key = key.into();
		let request = if let Some(chk) = chk {
			self.build_put_request(key, val)
				.condition_expression("attribute_exists(pk) and #value = :value")
				.expression_attribute_names("#value", "value")
				.expression_attribute_values(":value", AttributeValue::B(Blob::new(chk.into())))
		} else {
			self.build_put_request(key, val).condition_expression("attribute_not_exists(pk)")
		};

		request.send().await.map_err(|err| {
			let err = err.into_service_error();
			if let PutItemError::ConditionalCheckFailedException(_) = err {
				Error::TxConditionNotMet
			} else {
				Error::Ds(err.to_string())
			}
		})?;

		// Return result
		Ok(())
	}

	/// Delete a key
	pub async fn del<K>(&mut self, key: K) -> Result<(), Error>
	where
		K: Into<Key>,
	{
		// Check to see if transaction is closed
		if self.ok {
			return Err(Error::TxFinished);
		}
		// Check to see if transaction is writable
		if !self.rw {
			return Err(Error::TxReadonly);
		}

		let request = self.build_delete_request(key);
		request.send().await.map_err(|err| Error::Ds(err.to_string()))?;

		// Return result
		Ok(())
	}

	/// Delete a key
	pub async fn delc<K, V>(&mut self, key: K, chk: Option<V>) -> Result<(), Error>
	where
		K: Into<Key>,
		V: Into<Val>,
	{
		// Check to see if transaction is closed
		if self.ok {
			return Err(Error::TxFinished);
		}
		// Check to see if transaction is writable
		if !self.rw {
			return Err(Error::TxReadonly);
		}

		let request = if let Some(chk) = chk {
			self.build_delete_request(key)
				.condition_expression("attribute_exists(pk) and #value = :value")
				.expression_attribute_names("#value", "value")
				.expression_attribute_values(":value", AttributeValue::B(Blob::new(chk.into())))
		} else {
			self.build_delete_request(key).condition_expression("attribute_not_exists(pk)")
		};
		request.send().await.map_err(|err| {
			let err = err.into_service_error();
			if let DeleteItemError::ConditionalCheckFailedException(_) = err {
				Error::TxConditionNotMet
			} else {
				Error::Ds(err.to_string())
			}
		})?;

		// Return result
		Ok(())
	}

	/// Retrieve a range of keys from the databases
	pub async fn scan<K>(&mut self, rng: Range<K>, limit: u32) -> Result<Vec<(Key, Val)>, Error>
	where
		K: Into<Key>,
	{
		// Check to see if transaction is closed
		if self.ok {
			return Err(Error::TxFinished);
		}
		let from = rng.start.into();
		let to = rng.end.into();
		if to.cmp(&from) == Ordering::Less {
			return Ok(Vec::with_capacity(0));
		}
		// Scan the keys
		let from = AttributeValue::B(Blob::new(from));
		let to = AttributeValue::B(Blob::new(to));

		let (tx, mut rx) = tokio::sync::mpsc::channel::<Result<Vec<Key>, Error>>(10);
		for bucket in 0u8..self.shards {
			let tx = tx.clone();
			let client = Arc::clone(&self.client);
			let table = Arc::clone(&self.table);
			let f = from.clone();
			let t = to.clone();

			tokio::spawn(async move {
				let query = client
					.query()
					.table_name(table.as_ref())
					.index_name("GSI1")
					.key_condition_expression("#gsi1pk = :gsi1pk and #gsi1sk between :from and :to")
					// a BETWEEN b AND c — true if a is greater than or equal to b, and less than or equal to c.
					// We don't want: or equal to c
					.filter_expression("#pk < :to")
					.expression_attribute_names("#gsi1pk", "gsi1pk")
					.expression_attribute_names("#gsi1sk", "gsi1sk")
					.expression_attribute_names("#pk", "pk")
					.expression_attribute_values(
						":gsi1pk",
						AttributeValue::B(Blob::new(vec![bucket])),
					)
					.expression_attribute_values(":from", f)
					.expression_attribute_values(":to", t)
					.limit(limit as i32);
				let keys = query
					.send()
					.await
					.map(|res| {
						res.items.map_or(vec![], |items| {
							items
								.into_iter()
								.map(|mut item| {
									let key_att = item.remove("pk").expect("pk is defined in item");
									let key = match key_att {
										AttributeValue::B(blob) => blob.into_inner(),
										_ => unreachable!("key is not a blob"),
									};
									key
								})
								.collect::<Vec<_>>()
						})
					})
					.map_err(|err| Error::Ds(err.to_string()));
				tx.send(keys).await.expect("Response from DynamoDB is processed");
			});
		}
		drop(tx);
		let keys = {
			let mut keys = Vec::new();
			while let Some(response) = rx.recv().await {
				keys.extend(response?);
			}
			keys.sort();
			// drop irrelevant keys
			keys.into_iter().take(limit as usize).collect::<Vec<_>>()
		};

		let chunks = keys.chunks(40).map(|keys| {
			keys.iter().fold(KeysAndAttributes::builder(), |acc, key| {
				let key = AttributeValue::B(Blob::new(key.as_slice()));
				acc.keys(HashMap::from([("pk".to_string(), key.clone()), ("sk".to_string(), key)]))
			})
		});
		let (tx, mut rx) = tokio::sync::mpsc::channel::<Result<Vec<(Key, Val)>, Error>>(40);
		for chunk in chunks {
			let tx = tx.clone();
			let client = Arc::clone(&self.client);
			let table = Arc::clone(&self.table);
			tokio::spawn(async move {
				let items = client
					.batch_get_item()
					.request_items(table.as_ref(), chunk.build())
					.send()
					.await
					.map(|res| {
						res.responses
							.map(|mut tables| {
								tables.remove(table.as_ref()).expect("SurrealDB is exists")
							})
							.map_or(vec![], |items| {
								items
									.into_iter()
									.map(|mut item| {
										let key_att =
											item.remove("pk").expect("pk is defined in item");
										let val_att =
											item.remove("value").expect("value is defined in item");
										let key = match key_att {
											AttributeValue::B(blob) => blob.into_inner(),
											_ => unreachable!("pk is not a blob"),
										};
										let value = match val_att {
											AttributeValue::B(blob) => blob.into_inner(),
											_ => unreachable!("value is not a blob"),
										};
										(key, value)
									})
									.collect::<Vec<_>>()
							})
					})
					.map_err(|err| Error::Ds(err.to_string()));
				tx.send(items).await.expect("Response from DynamoDB is processed");
			});
		}
		drop(tx);
		let mut items = Vec::new();
		while let Some(response) = rx.recv().await {
			items.extend(response?);
		}
		items.sort_by(|a, b| a.cmp(b));
		Ok(items)
	}
}
