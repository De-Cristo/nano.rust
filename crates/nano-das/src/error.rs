use std::io;

#[derive(Debug)]
pub enum DasError {
    InvalidDatasetName { dataset: String, reason: String },
    MissingDasgoClient,
    DasgoClientFailed { status: Option<i32>, stderr: String },
    EmptyDataset { dataset: String, query: String },
    ParseError { line: String, reason: String },
    DuplicateFile { lfn: String },
    ManifestSchemaUnsupported { found: u32, supported: Vec<u32> },
    Io(io::Error),
}

impl std::fmt::Display for DasError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidDatasetName { dataset, reason } => {
                write!(f, "invalid dataset name '{}': {}", dataset, reason)
            }
            Self::MissingDasgoClient => write!(f, "dasgoclient not found"),
            Self::DasgoClientFailed { status, stderr } => {
                write!(f, "dasgoclient failed with status {:?}: {}", status, stderr)
            }
            Self::EmptyDataset { dataset, query } => {
                write!(f, "dataset '{}' is empty (query: {})", dataset, query)
            }
            Self::ParseError { line, reason } => {
                write!(f, "parse error on line '{}': {}", line, reason)
            }
            Self::DuplicateFile { lfn } => write!(f, "duplicate file in dataset: {}", lfn),
            Self::ManifestSchemaUnsupported { found, supported } => write!(
                f,
                "unsupported manifest schema {}: supported are {:?}",
                found, supported
            ),
            Self::Io(err) => write!(f, "I/O error: {}", err),
        }
    }
}

impl std::error::Error for DasError {}

impl From<io::Error> for DasError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

pub type Result<T> = std::result::Result<T, DasError>;
