use crate::model::{CmsFile, DatasetManifest, DatasetProvenance, DatasetRequest, FileSource};
use crate::resolver::DatasetResolver;
use crate::source_policy::SourcePolicy;
use crate::Result;

pub struct FixtureResolver {
    pub lfns: Vec<String>,
}

impl DatasetResolver for FixtureResolver {
    fn resolve(&self, request: &DatasetRequest) -> Result<DatasetManifest> {
        crate::model::validate_dataset_name(&request.dataset)?;

        let mut files = Vec::new();
        let limit = request.max_files.unwrap_or(self.lfns.len());

        let policy = SourcePolicy::GlobalXrootd;

        for lfn in self.lfns.iter().take(limit) {
            files.push(CmsFile {
                lfn: lfn.clone(),
                source: FileSource {
                    lfn: lfn.clone(),
                    global_xrootd_url: policy.normalize_lfn(lfn),
                    preferred_url: None,
                    local_path: None,
                },
                events: None,
                size_bytes: None,
                checksum: None,
                runs: vec![],
                lumis: vec![],
            });
        }

        let manifest = DatasetManifest {
            schema_version: 1,
            dataset: request.dataset.clone(),
            instance: request.instance.clone(),
            files,
            provenance: DatasetProvenance {
                resolver: "FixtureResolver".to_string(),
                query: "fixture".to_string(),
                resolved_at_utc: "1970-01-01T00:00:00Z".to_string(),
                command_or_endpoint: "fixture".to_string(),
                nano_das_version: "0.1.0".to_string(),
            },
        };

        manifest.validate()?;
        Ok(manifest)
    }
}
