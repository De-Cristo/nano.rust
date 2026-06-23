use crate::model::{
    CmsFile, DasInstance, DatasetManifest, DatasetProvenance, DatasetRequest, FileSource,
};
use crate::resolver::DatasetResolver;
use crate::source_policy::SourcePolicy;
use crate::{DasError, Result};
use chrono::Utc;
use std::process::Command;

pub struct DasgoClientResolver;

impl DatasetResolver for DasgoClientResolver {
    fn resolve(&self, request: &DatasetRequest) -> Result<DatasetManifest> {
        if request.require_valid {
            crate::model::validate_dataset_name(&request.dataset)?;
        }

        let instance_str = match &request.instance {
            DasInstance::ProdGlobal => "prod/global",
            DasInstance::ProdPhys03 => "prod/phys03",
            DasInstance::Custom(s) => s.as_str(),
        };

        let query = format!("file dataset={} instance={}", request.dataset, instance_str);

        let mut cmd = Command::new("dasgoclient");
        cmd.arg("-query").arg(&query);

        let output = cmd.output().map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                DasError::MissingDasgoClient
            } else {
                DasError::Io(e)
            }
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(DasError::DasgoClientFailed {
                status: output.status.code(),
                stderr,
            });
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut files = Vec::new();
        let policy = SourcePolicy::GlobalXrootd;

        for line in stdout.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if !line.starts_with("/store/") {
                return Err(DasError::ParseError {
                    line: line.to_string(),
                    reason: "LFN must start with /store/".to_string(),
                });
            }

            files.push(CmsFile {
                lfn: line.to_string(),
                source: FileSource {
                    lfn: line.to_string(),
                    global_xrootd_url: policy.normalize_lfn(line),
                    preferred_url: None,
                    local_path: None,
                },
                events: None,
                size_bytes: None,
                checksum: None,
                runs: vec![],
                lumis: vec![],
            });

            if let Some(max) = request.max_files {
                if files.len() >= max {
                    break;
                }
            }
        }

        if files.is_empty() {
            return Err(DasError::EmptyDataset {
                dataset: request.dataset.clone(),
                query,
            });
        }

        let manifest = DatasetManifest {
            schema_version: 1,
            dataset: request.dataset.clone(),
            instance: request.instance.clone(),
            files,
            provenance: DatasetProvenance {
                resolver: "DasgoClientResolver".to_string(),
                query: query.clone(),
                resolved_at_utc: Utc::now().to_rfc3339(),
                command_or_endpoint: "dasgoclient".to_string(),
                nano_das_version: env!("CARGO_PKG_VERSION").to_string(),
            },
        };

        manifest.validate()?;
        Ok(manifest)
    }
}
