use std::pin::Pin;

use arrow2::array::Array;
use arrow2::datatypes::{Field, Schema};
use arrow2::chunk::Chunk;
use futures::{Stream, StreamExt};

use crate::error::Result;

pub type DataBlock = Chunk<Box<dyn Array>>;
pub type DataBlockStream = Pin<Box<dyn Stream<Item = Result<DataBlock>> + Send + Sync + 'static>>;

pub fn schema_projected(schema: Schema, field_names: Vec<String>) -> Schema {
    // TODO: should validate that all columns are actually present...
    let retained: Vec<Field> = schema
        .fields
        .into_iter()
        .filter(|f| field_names.contains(&f.name))
        .collect();
    Schema::from(retained)
}

pub async fn pretty_print(mut rbs: DataBlockStream, schema: Schema) {
    let names = schema.fields.iter().map(|f| &f.name).collect::<Vec<_>>();
    let mut all_record_batches = Vec::new();
    if let Some(rb) = rbs.next().await {
        if rb.is_ok() {
            all_record_batches.push(rb.unwrap());
        }
    }
    println!("results: ");
    println!(
        "{}",
        arrow2::io::print::write(&all_record_batches[..], &names)
    );
}