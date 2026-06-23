use crate::model::DatasetManifest;
use crate::Result;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

impl DatasetManifest {
    pub fn read_json(path: impl AsRef<Path>) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let manifest: Self =
            serde_json::from_str(&content).map_err(|e| crate::DasError::ParseError {
                line: "".to_string(),
                reason: format!("JSON parse error: {}", e),
            })?;

        manifest.validate()?;
        Ok(manifest)
    }

    pub fn write_json(&self, path: impl AsRef<Path>) -> Result<()> {
        let content =
            serde_json::to_string_pretty(self).map_err(|e| crate::DasError::ParseError {
                line: "".to_string(),
                reason: format!("JSON format error: {}", e),
            })?;
        fs::write(path, content)?;
        Ok(())
    }

    pub fn validate(&self) -> Result<()> {
        if self.schema_version != 1 {
            return Err(crate::DasError::ManifestSchemaUnsupported {
                found: self.schema_version,
                supported: vec![1],
            });
        }
        crate::model::validate_dataset_name(&self.dataset)?;

        let mut seen = HashSet::new();
        for file in &self.files {
            if !file.lfn.starts_with("/store/") {
                return Err(crate::DasError::ParseError {
                    line: file.lfn.clone(),
                    reason: "LFN must start with /store/".to_string(),
                });
            }
            if !seen.insert(&file.lfn) {
                return Err(crate::DasError::DuplicateFile {
                    lfn: file.lfn.clone(),
                });
            }
        }
        Ok(())
    }
}
