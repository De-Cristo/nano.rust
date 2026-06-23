use crate::model::{DatasetManifest, DatasetRequest};
use crate::Result;

pub trait DatasetResolver {
    fn resolve(&self, request: &DatasetRequest) -> Result<DatasetManifest>;
}
