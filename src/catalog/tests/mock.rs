use std::collections::{BTreeMap, HashMap};
use std::fmt::{Display, Formatter};
use std::ops::RangeInclusive;
use std::str::FromStr;
use std::sync::Arc;

use async_stream::stream;
use catalog::error::Error;
use catalog::remote::{Kv, KvBackend, ValueIter};
use common_catalog::consts::{DEFAULT_CATALOG_NAME, DEFAULT_SCHEMA_NAME};
use common_recordbatch::RecordBatch;
use common_telemetry::logging::info;
use datatypes::data_type::ConcreteDataType;
use datatypes::schema::{ColumnSchema, Schema};
use datatypes::vectors::StringVector;
use serde::Serializer;
use table::engine::{EngineContext, TableEngine};
use table::metadata::TableId;
use table::requests::{AlterTableRequest, CreateTableRequest, DropTableRequest, OpenTableRequest};
use table::TableRef;
use tokio::sync::RwLock;

#[derive(Default)]
pub struct MockKvBackend {
    map: RwLock<BTreeMap<Vec<u8>, Vec<u8>>>,
}

impl Display for MockKvBackend {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        futures::executor::block_on(async {
            let map = self.map.read().await;
            for (k, v) in map.iter() {
                f.serialize_str(&String::from_utf8_lossy(k))?;
                f.serialize_str(" -> ")?;
                f.serialize_str(&String::from_utf8_lossy(v))?;
                f.serialize_str("\n")?;
            }
            Ok(())
        })
    }
}

#[async_trait::async_trait]
impl KvBackend for MockKvBackend {
    fn range<'a, 'b>(&'a self, key: &[u8]) -> ValueIter<'b, Error>
    where
        'a: 'b,
    {
        let prefix = key.to_vec();
        let prefix_string = String::from_utf8_lossy(&prefix).to_string();
        Box::pin(stream!({
            let maps = self.map.read().await.clone();
            for (k, v) in maps.range(prefix.clone()..) {
                let key_string = String::from_utf8_lossy(k).to_string();
                let matches = key_string.starts_with(&prefix_string);
                if matches {
                    yield Ok(Kv(k.clone(), v.clone()))
                } else {
                    info!("Stream finished");
                    return;
                }
            }
        }))
    }

    async fn set(&self, key: &[u8], val: &[u8]) -> Result<(), Error> {
        let mut map = self.map.write().await;
        map.insert(key.to_vec(), val.to_vec());
        Ok(())
    }

    async fn delete_range(&self, key: &[u8], end: &[u8]) -> Result<(), Error> {
        let start = key.to_vec();
        let end = end.to_vec();
        let range = RangeInclusive::new(start, end);

        let mut map = self.map.write().await;
        map.retain(|k, _| !range.contains(k));
        Ok(())
    }
}

#[derive(Default)]
pub struct MockTableEngine {
    tables: RwLock<HashMap<String, TableRef>>,
}

#[async_trait::async_trait]
impl TableEngine for MockTableEngine {
    fn name(&self) -> &str {
        "MockTableEngine"
    }

    /// Create a table with only one column
    async fn create_table(
        &self,
        _ctx: &EngineContext,
        request: CreateTableRequest,
    ) -> table::Result<TableRef> {
        let table_name = request.table_name.clone();
        let catalog_name = request
            .catalog_name
            .clone()
            .unwrap_or_else(|| DEFAULT_CATALOG_NAME.to_string());
        let schema_name = request
            .schema_name
            .clone()
            .unwrap_or_else(|| DEFAULT_SCHEMA_NAME.to_string());

        let default_table_id = "0".to_owned();
        let table_id = TableId::from_str(
            request
                .table_options
                .get("table_id")
                .unwrap_or(&default_table_id),
        )
        .unwrap();
        let schema = Arc::new(Schema::new(vec![ColumnSchema::new(
            "name",
            ConcreteDataType::string_datatype(),
            true,
        )]));

        let data = vec![Arc::new(StringVector::from(vec!["a", "b", "c"])) as _];
        let record_batch = RecordBatch::new(schema, data).unwrap();
        let table: TableRef = Arc::new(test_util::MemTable::new_with_catalog(
            &table_name,
            record_batch,
            table_id,
            catalog_name,
            schema_name,
        )) as Arc<_>;

        let mut tables = self.tables.write().await;
        tables.insert(table_name, table.clone() as TableRef);
        Ok(table)
    }

    async fn open_table(
        &self,
        _ctx: &EngineContext,
        request: OpenTableRequest,
    ) -> table::Result<Option<TableRef>> {
        Ok(self.tables.read().await.get(&request.table_name).cloned())
    }

    async fn alter_table(
        &self,
        _ctx: &EngineContext,
        _request: AlterTableRequest,
    ) -> table::Result<TableRef> {
        unimplemented!()
    }

    fn get_table(&self, _ctx: &EngineContext, name: &str) -> table::Result<Option<TableRef>> {
        futures::executor::block_on(async { Ok(self.tables.read().await.get(name).cloned()) })
    }

    fn table_exists(&self, _ctx: &EngineContext, name: &str) -> bool {
        futures::executor::block_on(async { self.tables.read().await.contains_key(name) })
    }

    async fn drop_table(
        &self,
        _ctx: &EngineContext,
        _request: DropTableRequest,
    ) -> table::Result<()> {
        unimplemented!()
    }
}
