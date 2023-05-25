#![cfg(feature = "kv-dynamodb")]

use crate::err::Error;
use crate::kvs::Key;
use crate::kvs::Val;
use aws_sdk_dynamodb::operation::delete_item::builders::DeleteItemFluentBuilder;
use aws_sdk_dynamodb::operation::delete_item::DeleteItemError;
use aws_sdk_dynamodb::operation::put_item::builders::PutItemFluentBuilder;
use aws_sdk_dynamodb::operation::put_item::PutItemError;
use aws_sdk_dynamodb::primitives::Blob;
use aws_sdk_dynamodb::types::AttributeValue;
use aws_sdk_dynamodb::Client;
use std::ops::Range;
use std::sync::Arc;

fn bucket(key: &[u8]) -> Vec<u8> {
	let mut result = Vec::new();
	let mut zero_count = 0;

	for &num in key.iter() {
		result.push(num);
		if num == 0 {
			zero_count += 1;
			if zero_count == 3 {
				break;
			}
		}
	}

	result
}

///
/// Datastore for DynamoDB key-value store
/// This Datastore does not support transactions
///
/// Requirements on table:
/// - Partition key is `Binary` with name `pk`
/// - Sort key is `Binary` with name `key`
///
/// Requirement os index:
/// - index name: `GSI1`
/// - Partition key is `Binary` with name `bucket`
/// - Sort key is `Binary` with name `key`
///
pub struct Datastore {
	client: Arc<Client>,
	table: Arc<String>,
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
}

impl Datastore {
	/// Open a new database from ENV
	pub async fn new(table: &str) -> Result<Datastore, Error> {
		let config = aws_config::load_from_env().await;
		let client = Arc::new(Client::new(&config));
		Ok(Datastore {
			client,
			table: Arc::new(String::from(table)),
		})
	}
	/// Start a new transaction
	pub async fn transaction(&self, write: bool, _: bool) -> Result<Transaction, Error> {
		Ok(Transaction {
			ok: false,
			rw: write,
			client: Arc::clone(&self.client),
			table: Arc::clone(&self.table),
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
		let bucket = bucket(&key);
		let key = AttributeValue::B(Blob::new(key));
		self.client
			.put_item()
			.table_name(self.table.as_ref())
			.item("pk", key.clone())
			.item("key", key)
			.item("value", AttributeValue::B(Blob::new(val.into())))
			.item("bucket", AttributeValue::B(Blob::new(bucket)))
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
			.key("key", key)
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
			.key("key", key)
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
			.key("key", key)
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
		// Scan the keys
		let rng_start = rng.start.into();
		let rng_end = rng.end.into();
		let bucket = bucket(&rng_start);
		let from = AttributeValue::B(Blob::new(rng_start));
		let to = AttributeValue::B(Blob::new(rng_end));
		let res = self
			.client
			.query()
			.table_name(self.table.as_ref())
			.index_name("GSI1")
			.key_condition_expression("#bucket = :bucket and #key between :from and :to")
			.expression_attribute_names("#bucket", "bucket")
			.expression_attribute_names("#key", "key")
			.expression_attribute_values(":bucket", AttributeValue::B(Blob::new(bucket)))
			.expression_attribute_values(":from", from)
			.expression_attribute_values(":to", to)
			.limit(limit as i32)
			.send()
			.await
			.map_err(|err| Error::Ds(err.to_string()))?;
		// Return result
		let items = res.items.map_or(vec![], |items| {
			items
				.into_iter()
				.map(|mut item| {
					let key_att = item.remove("key").expect("key is defined in item");
					let value_att = item.remove("value").expect("value is defined in item");
					let key = match key_att {
						AttributeValue::B(blob) => blob.into_inner(),
						_ => unreachable!("key is not a blob"),
					};

					let value = match value_att {
						AttributeValue::B(blob) => blob.into_inner(),
						_ => unreachable!("value is not a blob"),
					};
					(key, value)
				})
				.collect::<Vec<(Key, Val)>>()
		});
		Ok(items)
	}
}
