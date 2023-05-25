/// Enables `strict` server mode
#[cfg(any(
	feature = "kv-mem",
	feature = "kv-tikv",
	feature = "kv-rocksdb",
	feature = "kv-fdb",
	feature = "kv-indxdb",
	feature = "kv-dynamodb",
))]
#[derive(Debug)]
pub struct Strict;
