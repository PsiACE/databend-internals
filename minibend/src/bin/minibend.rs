use minibend::datablock::pretty_print;
use minibend::error::*;
use minibend::catalog::Catalog;

#[tokio::main]
async fn main() -> Result<()> {
    let test_file = format!("tests/source/alltypes_plain.parquet");
    let mut catalog = Catalog::default();
    catalog.add_parquet_table("parquet", &test_file)?;
    let table = catalog.get_table("parquet")?;

    let rbs = table.scan(None);
    let schema = table.schema();

    pretty_print(rbs, schema).await;

    Ok(())
}