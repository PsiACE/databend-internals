use std::sync::Arc;

use arrow2::datatypes::Schema;

use crate::datablock::DataBlockStream;

pub mod parquet;

pub type TableRef = Arc<dyn DataSource>;

pub trait DataSource {
    /// Returns the schema of the underlying data
    fn schema(&self) -> Schema;

    /// Returns a stream of DataBlocks
    fn scan(&self, projection: Option<Vec<String>>) -> DataBlockStream;
}
