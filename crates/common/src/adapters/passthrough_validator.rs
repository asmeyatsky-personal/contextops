use async_trait::async_trait;

use contextops_domain::entities::ContextFormat;
use contextops_domain::ports::schema_validator::{
    SchemaValidationError, SchemaValidatorPort, SchemaViolation,
};

/// Passthrough schema validator that accepts all content.
/// Used for development. Production would validate CLAUDE.md structure,
/// JSON schemas, YAML schemas, etc.
#[derive(Debug, Clone)]
pub struct PassthroughSchemaValidator;

impl PassthroughSchemaValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PassthroughSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SchemaValidatorPort for PassthroughSchemaValidator {
    async fn validate(
        &self,
        content: &[u8],
        format: ContextFormat,
    ) -> Result<Vec<SchemaViolation>, SchemaValidationError> {
        // Basic structural validation
        let mut violations = Vec::new();

        match format {
            ContextFormat::Json => {
                if serde_json::from_slice::<serde_json::Value>(content).is_err() {
                    violations.push(SchemaViolation {
                        path: "/".into(),
                        message: "Invalid JSON syntax".into(),
                        severity: contextops_domain::ports::schema_validator::ViolationSeverity::Error,
                    });
                }
            }
            _ => {
                // Passthrough for other formats
            }
        }

        Ok(violations)
    }
}
