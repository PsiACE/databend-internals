use std::collections::HashMap;

use crate::error::{Error, Result};
use crate::source::parquet::ParquetTable;
use crate::source::TableRef;

#[derive(Default)]
pub struct Catalog {
    tables: HashMap<String, TableRef>,
}

impl Catalog {
    /// Add parquet table
    pub fn add_parquet_table(&mut self, table: &str, path: &str) -> Result<()> {
        let source = ParquetTable::create(path.into())?;
        self.tables.insert(table.into(), source);
        Ok(())
    }

    /// Get table
    pub fn get_table(&self, table: &str) -> Result<TableRef> {
        self.tables
            .get(table)
            .cloned()
            .ok_or_else(|| Error::NoSuchTable(format!("Unable to get table named: {}", table)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Result;
    use futures::StreamExt;

    #[tokio::test]
    async fn test_format_parquet() -> Result<()> {
        let test_file = format!("tests/source/alltypes_plain.parquet");
        let mut catalog = Catalog::default();
        catalog.add_parquet_table("parquet", &test_file)?;
        let table = catalog.get_table("parquet")?;

        let mut rbs = table.scan(None);
        let mut actual_row_count = 0;

        if let Some(rrb) = rbs.next().await {
            let rb = rrb?;
            assert_eq!(rb.columns().len(), 11); // all columns are expected
            actual_row_count += rb.columns().get(0).unwrap().len();
        }
        assert_eq!(actual_row_count, 8);

        Ok(())
    }
}
