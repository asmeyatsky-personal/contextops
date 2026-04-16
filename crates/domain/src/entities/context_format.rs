use serde::{Deserialize, Serialize};

/// Supported context artifact formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ContextFormat {
    ClaudeMd,
    Json,
    Yaml,
    PlainText,
    PromptTemplate,
}

impl ContextFormat {
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "md" => Some(Self::ClaudeMd),
            "json" => Some(Self::Json),
            "yaml" | "yml" => Some(Self::Yaml),
            "txt" => Some(Self::PlainText),
            "prompt" | "tmpl" => Some(Self::PromptTemplate),
            _ => None,
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            Self::ClaudeMd => "md",
            Self::Json => "json",
            Self::Yaml => "yaml",
            Self::PlainText => "txt",
            Self::PromptTemplate => "prompt",
        }
    }
}
