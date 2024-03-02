#![cfg(feature = "kv-dynamodb")]

use crate::err::Error;
use crate::kvs::Val;
use crate::kvs::{Check, Key};
use crate::vs::{try_to_u64_be, u64_to_versionstamp, Versionstamp};
use aws_sdk_dynamodb::operation::delete_item::builders::DeleteItemFluentBuilder;
use aws_sdk_dynamodb::operation::delete_item::DeleteItemError;
use aws_sdk_dynamodb::operation::put_item::builders::PutItemFluentBuilder;
use aws_sdk_dynamodb::operation::put_item::PutItemError;
use aws_sdk_dynamodb::primitives::Blob;
use aws_sdk_dynamodb::types::{
	AttributeValue, Get, KeysAndAttributes, TransactGetItem, TransactWriteItem, Update,
};
use aws_sdk_dynamodb::Client;
use rand::Rng;
use std::borrow::Cow;
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

#[derive(Debug)]
enum Partition<'a> {
	Namespace {
		ns: Cow<'a, str>,
	},
	Database {
		ns: Cow<'a, str>,
		db: Cow<'a, str>,
	},
	Table {
		ns: Cow<'a, str>,
		db: Cow<'a, str>,
		tb: Cow<'a, str>,
	},
	Scope {
		ns: Cow<'a, str>,
		db: Cow<'a, str>,
		sc: Cow<'a, str>,
	},
	Change {
		ns: Cow<'a, str>,
		db: Cow<'a, str>,
		ts: Cow<'a, str>,
	},
	Graph {
		ns: Cow<'a, str>,
		db: Cow<'a, str>,
		tb: Cow<'a, str>,
		id: Cow<'a, str>,
	},
	Index {
		ns: Cow<'a, str>,
		db: Cow<'a, str>,
		tb: Cow<'a, str>,
		ix: Cow<'a, str>,
	},
	Fulltext {
		ns: Cow<'a, str>,
		db: Cow<'a, str>,
		tb: Cow<'a, str>,
		ix: Cow<'a, str>,
	},
	Global,
}

impl<'a> Partition<'a> {
	fn new(key: &'a Key) -> Partition<'a> {
		let segments =
			key.split(|num| 0.eq(num)).filter(|segment| segment.len() > 0).collect::<Vec<_>>();
		match segments.as_slice() {
			[[b'/', b'*', ..], [b'*', ..], [b'*', ..], [b'+', ..], [b'!', ..], ..] => {
				Partition::Fulltext {
					ns: String::from_utf8_lossy(&segments[0][2..]),
					db: String::from_utf8_lossy(&segments[1][1..]),
					tb: String::from_utf8_lossy(&segments[2][1..]),
					ix: String::from_utf8_lossy(&segments[3][1..]),
				}
			}
			[[b'/', b'*', ..], [b'*', ..], [b'*', ..], [b'+', ..], ..] => Partition::Index {
				ns: String::from_utf8_lossy(&segments[0][2..]),
				db: String::from_utf8_lossy(&segments[1][1..]),
				tb: String::from_utf8_lossy(&segments[2][1..]),
				ix: String::from_utf8_lossy(&segments[3][1..]),
			},
			[[b'/', b'*', ..], [b'*', ..], [b'*', ..], [b'~', ..], ..] => Partition::Graph {
				ns: String::from_utf8_lossy(&segments[0][2..]),
				db: String::from_utf8_lossy(&segments[1][1..]),
				tb: String::from_utf8_lossy(&segments[2][1..]),
				id: String::from_utf8_lossy(&segments[4][1..]),
			},
			[[b'/', b'*', ..], [b'*', ..], [b'#', ..], ..] => Partition::Change {
				ns: String::from_utf8_lossy(&segments[0][2..]),
				db: String::from_utf8_lossy(&segments[1][1..]),
				ts: String::from_utf8_lossy(&segments[2][1..]),
			},
			[[b'/', b'*', ..], [b'*', ..], [0xb1, ..], [b'!', ..], ..]
			| [[b'/', b'*', ..], [b'*', ..], [0xb1, ..], ..] => Partition::Scope {
				ns: String::from_utf8_lossy(&segments[0][2..]),
				db: String::from_utf8_lossy(&segments[1][1..]),
				sc: String::from_utf8_lossy(&segments[2][1..]),
			},
			[[b'/', b'*', ..], [b'*', ..], [b'*', ..], [b'!', ..], ..]
			| [[b'/', b'*', ..], [b'*', ..], [b'*', ..], ..] => Partition::Table {
				ns: String::from_utf8_lossy(&segments[0][2..]),
				db: String::from_utf8_lossy(&segments[1][1..]),
				tb: String::from_utf8_lossy(&segments[2][1..]),
			},
			[[b'/', b'*', ..], [b'*', ..], [b'!', ..], ..]
			| [[b'/', b'+', ..], [b'*', ..], [b'!', ..], ..]
			| [[b'/', b'*', ..], [b'*', ..]] => Partition::Database {
				ns: String::from_utf8_lossy(&segments[0][2..]),
				db: String::from_utf8_lossy(&segments[1][1..]),
			},
			[[b'/', b'*', ..]] | [[b'/', b'+', ..]] => Partition::Namespace {
				ns: String::from_utf8_lossy(&segments[0][2..]),
			},
			_ => Partition::Global,
		}
	}

	fn key(&self, shard: u8) -> String {
		match self {
			Partition::Global => format!("global[{shard}]://", shard = shard),
			Partition::Database {
				ns,
				db,
			} => format!("db[{shard}]://{ns}/{db}", shard = shard, ns = ns, db = db),
			Partition::Namespace {
				ns,
			} => format!("ns[{shard}]://{ns}", ns = ns),
			Partition::Table {
				ns,
				db,
				tb,
			} => {
				format!("table[{shard}]://{ns}/{db}/{tb}", shard = shard, ns = ns, db = db, tb = tb)
			}
			Partition::Scope {
				ns,
				db,
				sc,
			} => {
				format!("scope[{shard}]://{ns}/{db}/{sc}", shard = shard, ns = ns, db = db, sc = sc)
			}
			Partition::Change {
				ns,
				db,
				ts,
			} => {
				format!(
					"change[{shard}]://{ns}/{db}/{ts}",
					shard = shard,
					ns = ns,
					db = db,
					ts = ts
				)
			}
			Partition::Graph {
				ns,
				db,
				tb,
				id,
			} => format!(
				"graph[{shard}]://{ns}/{db}/{tb}/{id}",
				shard = shard,
				ns = ns,
				db = db,
				tb = tb,
				id = id
			),
			Partition::Index {
				ns,
				db,
				tb,
				ix,
			} => format!(
				"index[{shard}]://{ns}/{db}/{tb}/{ix}",
				shard = shard,
				ns = ns,
				db = db,
				tb = tb,
				ix = ix,
			),
			Partition::Fulltext {
				ns,
				db,
				tb,
				ix,
			} => format!(
				"fulltext[{shard}]://{ns}/{db}/{tb}/{ix}",
				shard = shard,
				ns = ns,
				db = db,
				tb = tb,
				ix = ix
			),
		}
	}
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
	done: bool,
	// Is the transaction read+write?
	write: bool,
	// Should we check unhandled transactions?
	check: Check,
	// client
	client: Arc<Client>,
	// table
	table: Arc<String>,
	// number of shards
	shards: u8,
}

impl Drop for Transaction {
	fn drop(&mut self) {
		if !self.done && self.write {
			// Check if already panicking
			if std::thread::panicking() {
				return;
			}
			// Handle the behaviour
			match self.check {
				Check::None => {
					trace!("A transaction was dropped without being committed or cancelled");
				}
				Check::Warn => {
					warn!("A transaction was dropped without being committed or cancelled");
				}
				Check::Panic => {
					#[cfg(debug_assertions)]
					{
						let backtrace = std::backtrace::Backtrace::force_capture();
						if let std::backtrace::BacktraceStatus::Captured = backtrace.status() {
							println!("{}", backtrace);
						}
					}
					panic!("A transaction was dropped without being committed or cancelled");
				}
			}
		}
	}
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
		// Specify the check level
		#[cfg(not(debug_assertions))]
		let check = Check::Warn;
		#[cfg(debug_assertions)]
		let check = Check::None;
		// Create a new transaction
		Ok(Transaction {
			done: false,
			write,
			check,
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
		let partition = Partition::new(&key);
		let gsi1pk = partition.key(shard);
		let key = AttributeValue::B(Blob::new(key.clone()));
		let result = self
			.client
			.put_item()
			.table_name(self.table.as_ref())
			.item("pk", key.clone())
			.item("sk", key.clone())
			.item("value", AttributeValue::B(Blob::new(val.into())))
			.item("gsi1pk", AttributeValue::S(gsi1pk))
			.item("gsi1sk", key);
		result
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

	/// Behaviour if unclosed
	pub(crate) fn check_level(&mut self, check: Check) {
		self.check = check;
	}

	/// Check if closed
	pub fn closed(&self) -> bool {
		self.done
	}

	/// Cancel a transaction
	pub async fn cancel(&mut self) -> Result<(), Error> {
		// Check to see if transaction is closed
		if self.done {
			return Err(Error::TxFinished);
		}
		// Mark this transaction as done
		self.done = true;
		// Continue
		Ok(())
	}

	/// Commit a transaction
	pub async fn commit(&mut self) -> Result<(), Error> {
		// Check to see if transaction is closed
		if self.done {
			return Err(Error::TxFinished);
		}
		// Check to see if transaction is writable
		if !self.write {
			return Err(Error::TxReadonly);
		}
		// Mark this transaction as done
		self.done = true;

		// Continue
		Ok(())
	}

	/// Obtain a new change timestamp for a key
	/// which is replaced with the current timestamp when the transaction is committed.
	/// NOTE: This should be called when composing the change feed entries for this transaction,
	/// which should be done immediately before the transaction commit.
	/// That is to keep other transactions commit delay(pessimistic) or conflict(optimistic) as less as possible.
	#[allow(unused)]
	pub(crate) async fn get_timestamp<K>(&mut self, key: K) -> Result<Versionstamp, Error>
	where
		K: Into<Key>,
	{
		// Check to see if transaction is closed
		if self.done {
			return Err(Error::TxFinished);
		}
		// Write the timestamp to the "last-write-timestamp" key
		// to ensure that no other transactions can commit with older timestamps.
		let k: Key = key.into();
		let prev = self.get_version(k.clone()).await?;
		let ver = match prev {
			Some(prev) => {
				let slice = prev.as_slice();
				let res: Result<[u8; 10], Error> = match slice.try_into() {
					Ok(ba) => Ok(ba),
					Err(e) => {
						dbg!(&e);
						Err(Error::Ds(e.to_string()))
					}
				};
				let array = res?;
				let prev = try_to_u64_be(array)?;
				prev + 1
			}
			None => 1,
		};

		let verbytes = u64_to_versionstamp(ver);

		self.set_version(k, verbytes.to_vec()).await.map_err(|err| {
			dbg!(&err);
			Error::Ds(err.to_string())
		})?;
		// Return the uint64 representation of the timestamp as the result
		Ok(verbytes)
	}

	/// Obtain a new key that is suffixed with the change timestamp
	pub(crate) async fn get_versionstamped_key<K>(
		&mut self,
		ts_key: K,
		prefix: K,
		suffix: K,
	) -> Result<Vec<u8>, Error>
	where
		K: Into<Key>,
	{
		// Check to see if transaction is closed
		if self.done {
			return Err(Error::TxFinished);
		}
		// Check to see if transaction is writable
		if !self.write {
			return Err(Error::TxReadonly);
		}

		let ts_key: Key = ts_key.into();
		let prefix: Key = prefix.into();
		let suffix: Key = suffix.into();

		let ts = self.get_timestamp(ts_key.clone()).await?;
		let mut k: Vec<u8> = prefix.clone();
		k.append(&mut ts.to_vec());
		k.append(&mut suffix.clone());

		Ok(k)
	}

	/// Check if a key exists
	pub async fn exi<K>(&mut self, key: K) -> Result<bool, Error>
	where
		K: Into<Key>,
	{
		// Check to see if transaction is closed
		if self.done {
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
			.map_err(|err| {
				dbg!(&err);
				Error::Ds(err.into_service_error().to_string())
			})?;

		// Return result
		Ok(res.item().is_some())
	}

	/// Fetch a key from the database
	pub async fn get<K>(&mut self, key: K) -> Result<Option<Val>, Error>
	where
		K: Into<Key>,
	{
		// Check to see if transaction is closed
		if self.done {
			return Err(Error::TxFinished);
		}

		let key = key.into();
		let db_key = AttributeValue::B(Blob::new(key.clone()));
		// Get the key
		let res = self
			.client
			.get_item()
			.table_name(self.table.as_ref())
			.key("pk", db_key.clone())
			.key("sk", db_key)
			.send()
			.await
			.map_err(|err| {
				dbg!(&err);
				Error::Ds(err.into_service_error().to_string())
			})?;

		// Return result
		let data = res.item.map(move |mut data| {
			let val = data.remove("value").expect("Item must contains value attribute");
			if let AttributeValue::B(blob) = val {
				let val = blob.into_inner();
				val
			} else {
				unreachable!("Item must contains blob value attribute")
			}
		});
		Ok(data)
	}

	/// Fetch a version for a key from the database
	async fn get_version<K>(&mut self, key: K) -> Result<Option<Val>, Error>
	where
		K: Into<Key>,
	{
		let key = key.into();
		let key = AttributeValue::B(Blob::new(key));

		let get_action = Get::builder()
			.table_name(self.table.as_ref())
			.key("pk", key.clone())
			.key("sk", key)
			.projection_expression("version")
			.build()
			.map_err(|err| {
				dbg!(&err);
				Error::Ds(err.to_string())
			})?;

		// Get the key
		let transact_items = vec![TransactGetItem::builder().set_get(Some(get_action)).build()];
		let resp = self
			.client
			.transact_get_items()
			.set_transact_items(Some(transact_items))
			.send()
			.await
			.map_err(|err| {
				dbg!(&err);
				Error::Ds(err.into_service_error().to_string())
			})?;

		let data = resp
			.responses
			.and_then(|mut responses| responses.pop())
			.and_then(|response| response.item)
			.and_then(|mut data| data.remove("version"))
			.map(|version| {
				if let AttributeValue::B(blob) = version {
					blob.into_inner()
				} else {
					unreachable!("Item must contains blob value attribute")
				}
			});
		Ok(data)
	}

	/// Insert or update a version for a key in the database
	async fn set_version<K, V>(&mut self, key: K, version: V) -> Result<(), Error>
	where
		K: Into<Key>,
		V: Into<Val>,
	{
		// Check to see if transaction is closed
		if self.done {
			return Err(Error::TxFinished);
		}
		// Check to see if transaction is writable
		if !self.write {
			return Err(Error::TxReadonly);
		}

		let key = key.into();
		let key = AttributeValue::B(Blob::new(key));
		let version = version.into();
		let version = AttributeValue::B(Blob::new(version));

		let update_action = Update::builder()
			.table_name(self.table.as_ref())
			.key("pk", key.clone())
			.key("sk", key)
			.update_expression("SET version = :v")
			.expression_attribute_values(":v", version)
			.build()
			.map_err(|err| {
				dbg!(&err);
				Error::Ds(err.to_string())
			})?;

		let transact_items =
			vec![TransactWriteItem::builder().set_update(Some(update_action)).build()];

		self.client
			.transact_write_items()
			.set_transact_items(Some(transact_items))
			.send()
			.await
			.map_err(|err| {
				dbg!(&err);
				Error::Ds(err.into_service_error().to_string())
			})?;

		// Return result
		Ok(())
	}

	/// Insert or update a key in the database
	pub async fn set<K, V>(&mut self, key: K, val: V) -> Result<(), Error>
	where
		K: Into<Key>,
		V: Into<Val>,
	{
		// Check to see if transaction is closed
		if self.done {
			return Err(Error::TxFinished);
		}
		// Check to see if transaction is writable
		if !self.write {
			return Err(Error::TxReadonly);
		}
		// Set the key
		let request = self.build_put_request(key, val);
		request.send().await.map_err(|err| {
			dbg!(&err);
			Error::Ds(err.into_service_error().to_string())
		})?;

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
		if self.done {
			return Err(Error::TxFinished);
		}
		// Check to see if transaction is writable
		if !self.write {
			return Err(Error::TxReadonly);
		}
		// Set the key if not exists
		let request =
			self.build_put_request(key, val).condition_expression("attribute_not_exists(pk)");

		request.send().await.map_err(|err| {
			dbg!(&err);
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
		if self.done {
			return Err(Error::TxFinished);
		}
		// Check to see if transaction is writable
		if !self.write {
			return Err(Error::TxReadonly);
		}
		let key = key.into();
		let request = if let Some(chk) = chk {
			self.build_put_request(key, val)
				.condition_expression("attribute_exists(pk) and #check = :check")
				.expression_attribute_names("#check", "value")
				.expression_attribute_values(":check", AttributeValue::B(Blob::new(chk.into())))
		} else {
			self.build_put_request(key, val).condition_expression("attribute_not_exists(pk)")
		};

		request.send().await.map_err(|err| {
			dbg!(&err);
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
		if self.done {
			return Err(Error::TxFinished);
		}
		// Check to see if transaction is writable
		if !self.write {
			return Err(Error::TxReadonly);
		}

		let request = self.build_delete_request(key);
		request.send().await.map_err(|err| {
			dbg!(&err);
			Error::Ds(err.into_service_error().to_string())
		})?;

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
		if self.done {
			return Err(Error::TxFinished);
		}
		// Check to see if transaction is writable
		if !self.write {
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
			dbg!(&err);
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
		if self.done {
			return Err(Error::TxFinished);
		}
		let from = rng.start.into();
		let partition = Partition::new(&from);
		let to = rng.end.into();
		if to.cmp(&from) == Ordering::Less {
			return Ok(Vec::with_capacity(0));
		}
		// Scan the keys
		let from = AttributeValue::B(Blob::new(from.clone()));
		let to = AttributeValue::B(Blob::new(to));

		let (tx, mut rx) = tokio::sync::mpsc::channel::<Result<Vec<Key>, Error>>(10);
		for shard in 0u8..self.shards {
			let tx = tx.clone();
			let client = Arc::clone(&self.client);
			let table = Arc::clone(&self.table);
			let f = from.clone();
			let t = to.clone();
			let gsi1pk = partition.key(shard);

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
					.expression_attribute_values(":gsi1pk", AttributeValue::S(gsi1pk))
					.expression_attribute_values(":from", f)
					.expression_attribute_values(":to", t)
					.limit(std::cmp::min(limit, 1000) as i32);
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
					.map_err(|err| {
						dbg!(&err);
						Error::Ds(err.into_service_error().to_string())
					});
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
					.request_items(table.as_ref(), chunk.build().expect("Valid chunk"))
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
					.map_err(|err| {
						dbg!(&err, limit);
						Error::Ds(err.into_service_error().to_string())
					});
				tx.send(items).await.expect("Response from DynamoDB is processed");
			});
		}
		drop(tx);
		let mut items = Vec::new();
		while let Some(response) = rx.recv().await {
			items.extend(response?);
		}
		items.sort_by(|(a, _), (b, _)| a.cmp(b));
		Ok(items)
	}
}
