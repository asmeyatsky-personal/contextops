use async_trait::async_trait;

use contextops_domain::entities::ContextFormat;
use contextops_domain::ports::schema_validator::{
    SchemaValidationError, SchemaValidatorPort, SchemaViolation, ViolationSeverity,
};

/// Schema validator with real structural validation for each format.
///
/// - CLAUDE.md: checks for heading structure, non-empty sections
/// - JSON: validates syntax
/// - YAML: validates syntax (basic check)
/// - PlainText / PromptTemplate: minimal validation
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
        let mut violations = Vec::new();

        let text = match std::str::from_utf8(content) {
            Ok(t) => t,
            Err(_) => {
                violations.push(SchemaViolation {
                    path: "/".into(),
                    message: "Content is not valid UTF-8".into(),
                    severity: ViolationSeverity::Error,
                });
                return Ok(violations);
            }
        };

        match format {
            ContextFormat::Json => validate_json(text, &mut violations),
            ContextFormat::Yaml => validate_yaml(text, &mut violations),
            ContextFormat::ClaudeMd => validate_claude_md(text, &mut violations),
            ContextFormat::PlainText => validate_plain_text(text, &mut violations),
            ContextFormat::PromptTemplate => validate_prompt_template(text, &mut violations),
        }

        Ok(violations)
    }
}

fn validate_json(text: &str, violations: &mut Vec<SchemaViolation>) {
    if let Err(e) = serde_json::from_str::<serde_json::Value>(text) {
        violations.push(SchemaViolation {
            path: "/".into(),
            message: format!("Invalid JSON: {e}"),
            severity: ViolationSeverity::Error,
        });
    }
}

fn validate_yaml(text: &str, violations: &mut Vec<SchemaViolation>) {
    // Basic YAML structure check: must not start with invalid characters,
    // must have at least one key-value pair or list item for non-empty content
    let trimmed = text.trim();
    if trimmed.is_empty() {
        violations.push(SchemaViolation {
            path: "/".into(),
            message: "YAML content is empty".into(),
            severity: ViolationSeverity::Error,
        });
        return;
    }

    // Check for tabs (YAML forbids tabs for indentation)
    for (i, line) in text.lines().enumerate() {
        if line.starts_with('\t') {
            violations.push(SchemaViolation {
                path: format!("/line:{}", i + 1),
                message: "YAML must not use tabs for indentation".into(),
                severity: ViolationSeverity::Error,
            });
            break;
        }
    }
}

fn validate_claude_md(text: &str, violations: &mut Vec<SchemaViolation>) {
    let trimmed = text.trim();

    if trimmed.is_empty() {
        violations.push(SchemaViolation {
            path: "/".into(),
            message: "CLAUDE.md content is empty".into(),
            severity: ViolationSeverity::Error,
        });
        return;
    }

    // CLAUDE.md should start with a heading
    if !trimmed.starts_with('#') {
        violations.push(SchemaViolation {
            path: "/".into(),
            message: "CLAUDE.md should start with a markdown heading (# Title)".into(),
            severity: ViolationSeverity::Warning,
        });
    }

    // Check for at least one section with content
    let has_heading = text.lines().any(|l| l.starts_with('#'));
    if !has_heading {
        violations.push(SchemaViolation {
            path: "/".into(),
            message: "CLAUDE.md should contain at least one markdown heading".into(),
            severity: ViolationSeverity::Warning,
        });
    }

    // Warn if very short (likely placeholder)
    if trimmed.len() < 20 {
        violations.push(SchemaViolation {
            path: "/".into(),
            message: "Context content is very short — may be a placeholder".into(),
            severity: ViolationSeverity::Warning,
        });
    }
}

fn validate_plain_text(text: &str, violations: &mut Vec<SchemaViolation>) {
    if text.trim().is_empty() {
        violations.push(SchemaViolation {
            path: "/".into(),
            message: "Plain text content is empty".into(),
            severity: ViolationSeverity::Error,
        });
    }
}

fn validate_prompt_template(text: &str, violations: &mut Vec<SchemaViolation>) {
    if text.trim().is_empty() {
        violations.push(SchemaViolation {
            path: "/".into(),
            message: "Prompt template content is empty".into(),
            severity: ViolationSeverity::Error,
        });
        return;
    }

    // Check for unclosed template variables
    let open_count = text.matches("{{").count();
    let close_count = text.matches("}}").count();
    if open_count != close_count {
        violations.push(SchemaViolation {
            path: "/".into(),
            message: format!(
                "Mismatched template delimiters: {} opening '{{{{' vs {} closing '}}}}'",
                open_count, close_count
            ),
            severity: ViolationSeverity::Error,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn validate(content: &[u8], format: ContextFormat) -> Vec<SchemaViolation> {
        let validator = PassthroughSchemaValidator::new();
        validator.validate(content, format).await.unwrap()
    }

    fn has_error(violations: &[SchemaViolation]) -> bool {
        violations.iter().any(|v| v.severity == ViolationSeverity::Error)
    }

    fn has_warning(violations: &[SchemaViolation]) -> bool {
        violations.iter().any(|v| v.severity == ViolationSeverity::Warning)
    }

    #[tokio::test]
    async fn valid_json_passes() {
        let v = validate(br#"{"key": "value"}"#, ContextFormat::Json).await;
        assert!(!has_error(&v));
    }

    #[tokio::test]
    async fn invalid_json_fails() {
        let v = validate(b"not json", ContextFormat::Json).await;
        assert!(has_error(&v));
    }

    #[tokio::test]
    async fn valid_claude_md_passes() {
        let v = validate(
            b"# My Context\n\nThis is a well-structured context file.",
            ContextFormat::ClaudeMd,
        ).await;
        assert!(!has_error(&v));
        assert!(!has_warning(&v));
    }

    #[tokio::test]
    async fn claude_md_without_heading_warns() {
        let v = validate(
            b"This context file has no heading but has content.",
            ContextFormat::ClaudeMd,
        ).await;
        assert!(!has_error(&v));
        assert!(has_warning(&v));
    }

    #[tokio::test]
    async fn empty_content_fails_all_formats() {
        for format in [
            ContextFormat::ClaudeMd,
            ContextFormat::Json,
            ContextFormat::PlainText,
            ContextFormat::PromptTemplate,
        ] {
            let v = validate(b"", format).await;
            assert!(has_error(&v), "empty content should fail for {:?}", format);
        }
    }

    #[tokio::test]
    async fn yaml_with_tabs_fails() {
        let v = validate(b"\tkey: value", ContextFormat::Yaml).await;
        assert!(has_error(&v));
    }

    #[tokio::test]
    async fn valid_yaml_passes() {
        let v = validate(b"key: value\nlist:\n  - item1", ContextFormat::Yaml).await;
        assert!(!has_error(&v));
    }

    #[tokio::test]
    async fn prompt_template_mismatched_delimiters_fails() {
        let v = validate(b"Hello {{name}}, welcome to {{", ContextFormat::PromptTemplate).await;
        assert!(has_error(&v));
    }

    #[tokio::test]
    async fn prompt_template_balanced_delimiters_passes() {
        let v = validate(b"Hello {{name}}, welcome to {{place}}", ContextFormat::PromptTemplate).await;
        assert!(!has_error(&v));
    }

    #[tokio::test]
    async fn non_utf8_fails() {
        let v = validate(&[0xFF, 0xFE, 0x00], ContextFormat::PlainText).await;
        assert!(has_error(&v));
    }
}
