use std::pin::Pin;

use arrow2::array::Array;
use arrow2::chunk::Chunk;
use futures::Stream;

use crate::error::Result;

pub type DataBlock = Chunk<Box<dyn Array>>;
pub type DataBlockStream = Pin<Box<dyn Stream<Item = Result<DataBlock>> + Send + Sync + 'static>>;
