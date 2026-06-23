pub mod backends;
pub mod error;
pub mod manifest;
pub mod model;
pub mod resolver;
pub mod source_policy;

pub use backends::dasgoclient::DasgoClientResolver;
pub use backends::fixture::FixtureResolver;
pub use error::{DasError, Result};
pub use model::{CmsFile, DasInstance, DatasetManifest, DatasetRequest, FileSource};
pub use resolver::DatasetResolver;
pub use source_policy::SourcePolicy;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::fixture::FixtureResolver;

    #[test]
    fn test_valid_dataset_names() {
        assert!(model::validate_dataset_name("/Muon0/Run2024C-NanoAODv15-v1/NANOAOD").is_ok());
        assert!(model::validate_dataset_name("/ScoutingPFRun3/Run2024D-v1/RAW").is_ok());
    }

    #[test]
    fn test_invalid_dataset_names() {
        assert!(model::validate_dataset_name("").is_err());
        assert!(model::validate_dataset_name("Muon0/Run2024C/NANOAOD").is_err());
        assert!(model::validate_dataset_name("/Muon0/Run2024 C/NANOAOD").is_err());
        assert!(model::validate_dataset_name("/Muon0/Run2024C").is_err());
        assert!(model::validate_dataset_name("/Muon0/Run2024C/NANOAOD/EXTRA").is_err());
    }

    #[test]
    fn test_source_policy() {
        let policy = SourcePolicy::GlobalXrootd;
        assert_eq!(
            policy.normalize_lfn("/store/data/test.root"),
            "root://cms-xrd-global.cern.ch//store/data/test.root"
        );
    }

    #[test]
    fn test_fixture_resolver() {
        let resolver = FixtureResolver {
            lfns: vec![
                "/store/data/1.root".to_string(),
                "/store/data/2.root".to_string(),
            ],
        };

        let req = DatasetRequest {
            dataset: "/Test/Dataset/NANOAOD".to_string(),
            instance: DasInstance::ProdGlobal,
            max_files: None,
            run_range: None,
            require_valid: true,
        };

        let manifest = resolver.resolve(&req).unwrap();
        assert_eq!(manifest.files.len(), 2);
        assert_eq!(
            manifest.files[0].source.global_xrootd_url,
            "root://cms-xrd-global.cern.ch//store/data/1.root"
        );
    }

    #[test]
    fn test_fixture_resolver_max_files() {
        let resolver = FixtureResolver {
            lfns: vec![
                "/store/data/1.root".to_string(),
                "/store/data/2.root".to_string(),
                "/store/data/3.root".to_string(),
            ],
        };

        let req = DatasetRequest {
            dataset: "/Test/Dataset/NANOAOD".to_string(),
            instance: DasInstance::ProdGlobal,
            max_files: Some(2),
            run_range: None,
            require_valid: true,
        };

        let manifest = resolver.resolve(&req).unwrap();
        assert_eq!(manifest.files.len(), 2);
        assert_eq!(manifest.files[1].lfn, "/store/data/2.root");
    }

    #[test]
    fn test_manifest_duplicate_rejection() {
        let resolver = FixtureResolver {
            lfns: vec![
                "/store/data/1.root".to_string(),
                "/store/data/1.root".to_string(),
            ],
        };

        let req = DatasetRequest {
            dataset: "/Test/Dataset/NANOAOD".to_string(),
            instance: DasInstance::ProdGlobal,
            max_files: None,
            run_range: None,
            require_valid: true,
        };

        assert!(resolver.resolve(&req).is_err());
    }

    #[test]
    fn test_manifest_json_roundtrip() {
        let resolver = FixtureResolver {
            lfns: vec!["/store/data/1.root".to_string()],
        };
        let req = DatasetRequest {
            dataset: "/Test/Dataset/NANOAOD".to_string(),
            instance: DasInstance::ProdGlobal,
            max_files: None,
            run_range: None,
            require_valid: true,
        };
        let manifest = resolver.resolve(&req).unwrap();

        let dir = std::env::temp_dir();
        let path = dir.join("test_manifest.json");
        manifest.write_json(&path).unwrap();

        let loaded = DatasetManifest::read_json(&path).unwrap();
        assert_eq!(manifest, loaded);
    }
}
