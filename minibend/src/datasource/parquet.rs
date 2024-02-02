use std::fs::File;
use std::sync::Arc;

use arrow2::array::Array;
use arrow2::chunk::Chunk;
use arrow2::datatypes::Schema;
use arrow2::io::parquet::read::*;
use async_fn_stream::fn_stream;

use crate::datablock::DataBlockStream;
use crate::datasource::DataSource;
use crate::error::Result;

use super::TableRef;

#[derive(Debug, Clone)]
pub struct ParquetTable {
    pub path: String,
}

impl ParquetTable {
    pub fn create(path: String) -> Result<TableRef> {
        Ok(Arc::new(Self { path }))
    }

    fn get_reader(&self) -> Result<FileReader<File>> {
        let mut file = File::open(self.path.clone())?;
        let metadata = read_metadata(&mut file)?;
        let schema = infer_schema(&metadata)?;
        let reader = FileReader::new(file, metadata.row_groups, schema, None, None, None);
        Ok(reader)
    }
}

impl DataSource for ParquetTable {
    fn schema(&self) -> Schema {
        let reader = self.get_reader().unwrap();
        reader.schema().clone()
    }

    fn scan(&self, projection: Option<Vec<String>>) -> DataBlockStream {
        let reader = self.get_reader().unwrap();

        let indexes = projection.map(|projection| {
            projection
                .iter()
                .map(|p| {
                    self.schema()
                        .fields
                        .iter()
                        .enumerate()
                        .find(|(_idx, field)| field.name.eq(p))
                        .map(|(idx, _field)| idx)
                        .unwrap()
                })
                .collect::<Vec<_>>()
        });

        // need to consider only relevant columns
        let output = fn_stream(|emitter| async move {
            for maybe_chunk in reader {
                let chunk = maybe_chunk.unwrap();
                let result_chunk = match indexes {
                    Some(ref indexes) => {
                        let arrays = chunk.arrays();
                        let mut r: Vec<Box<dyn Array>> = Vec::new();
                        for idx in indexes {
                            let array = arrays.get(*idx).unwrap();
                            r.push(array.clone());
                        }
                        Chunk::new(r)
                    }
                    None => chunk,
                };
                // yield elements from stream via `emitter`
                emitter.emit(Ok(result_chunk)).await;
            }
        });

        Box::pin(output) as DataBlockStream
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow2::datatypes::{DataType, Field, TimeUnit};
    use futures::StreamExt;

    #[test]
    fn test_parquet_schema() -> Result<()> {
        let test_file = format!("tests/source/alltypes_plain.parquet");
        let parquet_datasource = ParquetTable::create(test_file)?;

        let actual_schema = parquet_datasource.schema();

        let expected_schema = Schema::from(vec![
            Field::new("id", DataType::Int32, true),
            Field::new("bool_col", DataType::Boolean, true),
            Field::new("tinyint_col", DataType::Int32, true),
            Field::new("smallint_col", DataType::Int32, true),
            Field::new("int_col", DataType::Int32, true),
            Field::new("bigint_col", DataType::Int64, true),
            Field::new("float_col", DataType::Float32, true),
            Field::new("double_col", DataType::Float64, true),
            Field::new("date_string_col", DataType::Binary, true),
            Field::new("string_col", DataType::Binary, true),
            Field::new(
                "timestamp_col",
                DataType::Timestamp(TimeUnit::Nanosecond, None),
                true,
            ),
        ]);

        assert_eq!(actual_schema, expected_schema);

        Ok(())
    }

    #[tokio::test]
    async fn test_scan_parquet_without_projection() -> Result<()> {
        let test_file = format!("tests/source/alltypes_plain.parquet");
        let parquet_datasource = ParquetTable::create(test_file.to_string())?;

        let mut rbs = parquet_datasource.scan(None);
        let mut actual_row_count = 0;

        if let Some(rrb) = rbs.next().await {
            let rb = rrb?;
            assert_eq!(rb.columns().len(), 11); // all columns are expected
            actual_row_count += rb.columns().get(0).unwrap().len();
        }
        assert_eq!(actual_row_count, 8);

        Ok(())
    }

    #[tokio::test]
    async fn test_scan_parquet_with_projection() -> Result<()> {
        let test_file = format!("tests/source/alltypes_plain.parquet");
        let parquet_datasource = ParquetTable::create(test_file.to_string())?;

        let mut rbs = parquet_datasource.scan(Some(vec!["id".to_string()]));
        let mut actual_row_count = 0;

        if let Some(rrb) = rbs.next().await {
            let rb = rrb?;
            assert_eq!(rb.columns().len(), 1); // only a single column.rs is requested
            actual_row_count += rb.columns().get(0).unwrap().len();
        }
        assert_eq!(actual_row_count, 8);

        Ok(())
    }
}
