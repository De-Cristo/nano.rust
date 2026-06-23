#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourcePolicy {
    LogicalOnly,
    GlobalXrootd,
    SiteXrootd { redirector: String },
    HttpFromTemplate { base_url: String },
}

impl SourcePolicy {
    pub fn normalize_lfn(&self, lfn: &str) -> String {
        match self {
            Self::LogicalOnly => lfn.to_string(),
            Self::GlobalXrootd => format!("root://cms-xrd-global.cern.ch/{}", lfn),
            Self::SiteXrootd { redirector } => format!("root://{}/{}", redirector, lfn),
            Self::HttpFromTemplate { base_url } => format!("{}{}", base_url, lfn),
        }
    }
}
