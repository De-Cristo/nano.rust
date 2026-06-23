use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DatasetRequest {
    pub dataset: String,
    pub instance: DasInstance,
    pub max_files: Option<usize>,
    pub run_range: Option<RunRange>,
    pub require_valid: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DasInstance {
    #[serde(rename = "prod/global")]
    ProdGlobal,
    #[serde(rename = "prod/phys03")]
    ProdPhys03,
    #[serde(untagged)]
    Custom(String),
}

impl std::fmt::Display for DasInstance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ProdGlobal => write!(f, "prod/global"),
            Self::ProdPhys03 => write!(f, "prod/phys03"),
            Self::Custom(s) => write!(f, "{}", s),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunRange {
    pub min_run: Option<u32>,
    pub max_run: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DatasetManifest {
    pub schema_version: u32,
    pub dataset: String,
    pub instance: DasInstance,
    pub files: Vec<CmsFile>,
    pub provenance: DatasetProvenance,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CmsFile {
    pub lfn: String,
    pub source: FileSource,
    pub events: Option<u64>,
    pub size_bytes: Option<u64>,
    pub checksum: Option<String>,
    pub runs: Vec<u32>,
    pub lumis: Vec<LumiRange>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileSource {
    pub lfn: String,
    pub global_xrootd_url: String,
    pub preferred_url: Option<String>,
    pub local_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LumiRange {
    pub run: u32,
    pub first_lumi: u32,
    pub last_lumi: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DatasetProvenance {
    pub resolver: String,
    pub query: String,
    pub resolved_at_utc: String,
    pub command_or_endpoint: String,
    pub nano_das_version: String,
}

pub fn validate_dataset_name(dataset: &str) -> crate::Result<()> {
    if dataset.is_empty() {
        return Err(crate::DasError::InvalidDatasetName {
            dataset: dataset.to_string(),
            reason: "empty string".to_string(),
        });
    }
    if !dataset.starts_with('/') {
        return Err(crate::DasError::InvalidDatasetName {
            dataset: dataset.to_string(),
            reason: "missing leading slash".to_string(),
        });
    }
    if dataset.chars().any(|c| c.is_whitespace()) {
        return Err(crate::DasError::InvalidDatasetName {
            dataset: dataset.to_string(),
            reason: "contains whitespace".to_string(),
        });
    }
    let parts: Vec<&str> = dataset.split('/').collect();
    if parts.len() != 4 || !parts[0].is_empty() {
        return Err(crate::DasError::InvalidDatasetName {
            dataset: dataset.to_string(),
            reason: "wrong number of path components, expected /PrimaryDataset/ProcessedDataset/DataTier".to_string(),
        });
    }
    Ok(())
}
