use std::collections::HashMap;

use regex::Regex;

use crate::configs::{
    EngineConfig, EngineParam, MatchType, ParamType, UnknownBehavior, Validation,
};
use crate::errors::ValidationError;
use crate::models::{RunRequest, ValidatedParam, ValidatedRunRequest};

/// Validates WES requests at the API layer before command building.
/// This ensures bad requests are rejected immediately with clear error
/// messages.
pub struct EngineRequestValidator {
    config: EngineConfig,
}

impl EngineRequestValidator {
    pub fn new(config: EngineConfig) -> Self {
        Self { config }
    }

    /// Validate a WES request and return validated request or error.
    /// This modifies the request to include validated engine parameters.
    pub fn validate(&self, request: &RunRequest) -> Result<ValidatedRunRequest, ValidationError> {
        // 0. Validate requested workflow/engine capabilities against this engine config
        self.validate_workflow_capabilities(request)?;

        // 1. Check denied parameters first (security check)
        self.check_denied_params(request.workflow_engine_parameters.as_ref())?;

        // 2. Filter out ignored parameters
        let filtered_params =
            self.filter_ignored_params(request.workflow_engine_parameters.as_ref());

        // 3. Validate and process engine parameters
        let validated_params = self.validate_engine_params(filtered_params.as_ref())?;

        Ok(ValidatedRunRequest {
            workflow_params: request.workflow_params.clone(),
            workflow_type: request.workflow_type.clone(),
            workflow_type_version: request.workflow_type_version.clone(),
            tags: request.tags.clone(),
            workflow_engine_parameters: if validated_params.is_empty() {
                None
            } else {
                Some(validated_params)
            },
            workflow_engine: request.workflow_engine.clone(),
            workflow_engine_version: request.workflow_engine_version.clone(),
            workflow_url: request.workflow_url.clone(),
        })
    }

    fn validate_workflow_capabilities(&self, request: &RunRequest) -> Result<(), ValidationError> {
        if !self
            .config
            .workflow_types
            .iter()
            .any(|supported| supported.eq_ignore_ascii_case(&request.workflow_type))
        {
            return Err(ValidationError::ValidationFailed(
                "workflow_type".to_string(),
                format!(
                    "Unsupported workflow_type '{}'. Supported: {}",
                    request.workflow_type,
                    self.config.workflow_types.join(", ")
                ),
            ));
        }

        if !self
            .config
            .workflow_type_versions
            .iter()
            .any(|supported| supported.eq_ignore_ascii_case(&request.workflow_type_version))
        {
            return Err(ValidationError::ValidationFailed(
                "workflow_type_version".to_string(),
                format!(
                    "Unsupported workflow_type_version '{}'. Supported: {}",
                    request.workflow_type_version,
                    self.config.workflow_type_versions.join(", ")
                ),
            ));
        }

        if !request.workflow_engine.eq_ignore_ascii_case(&self.config.name) {
            return Err(ValidationError::ValidationFailed(
                "workflow_engine".to_string(),
                format!(
                    "Unsupported workflow_engine '{}'. Expected '{}'",
                    request.workflow_engine, self.config.name
                ),
            ));
        }

        if request.workflow_engine_version != self.config.version {
            return Err(ValidationError::ValidationFailed(
                "workflow_engine_version".to_string(),
                format!(
                    "Unsupported workflow_engine_version '{}'. Expected '{}'",
                    request.workflow_engine_version, self.config.version
                ),
            ));
        }

        Ok(())
    }

    /// Check for explicitly denied parameters (security layer)
    fn check_denied_params(
        &self,
        params: Option<&HashMap<String, String>>,
    ) -> Result<(), ValidationError> {
        let Some(params) = params else {
            return Ok(());
        };

        for key in params.keys() {
            for denied in &self.config.denied_params {
                let matches = match denied.match_type {
                    MatchType::Exact => key == &denied.pattern,
                    MatchType::Regex => match Regex::new(&denied.pattern) {
                        Ok(re) => re.is_match(key),
                        Err(_) => {
                            tracing::warn!(
                                "Invalid regex pattern in denied params: {}",
                                denied.pattern
                            );
                            false
                        },
                    },
                };

                if matches {
                    return Err(ValidationError::DeniedParameter(
                        key.clone(),
                        denied.reason.clone(),
                    ));
                }
            }
        }
        Ok(())
    }

    /// Filter out ignored parameters without validation errors
    fn filter_ignored_params(
        &self,
        params: Option<&HashMap<String, String>>,
    ) -> Option<HashMap<String, String>> {
        let params = params?;

        let Some(ref ignored) = self.config.ignored_params else {
            return Some(params.clone());
        };

        let filtered: HashMap<_, _> = params
            .iter()
            .filter(|(k, _)| !ignored.contains(k))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        if filtered.is_empty() {
            None
        } else {
            Some(filtered)
        }
    }

    /// Main validation logic for engine parameters
    fn validate_engine_params(
        &self,
        params: Option<&HashMap<String, String>>,
    ) -> Result<Vec<ValidatedParam>, ValidationError> {
        let mut validated = Vec::new();
        let mut handled_names = std::collections::HashSet::new();

        // Process known parameters first
        for param_spec in &self.config.engine_params.validated_params {
            let user_input = self.find_user_input_for_param(params, param_spec);

            if let Some((name, value_str)) = user_input {
                // Mark this parameter name as handled
                handled_names.insert(name.clone());

                // Parse and validate the parameter
                let validated_param =
                    self.process_known_parameter(param_spec, &name, &value_str)?;
                validated.push(validated_param);
            } else {
                // Handle missing required parameters or strict defaults
                if param_spec.required {
                    return Err(ValidationError::RequiredMissing(param_spec.names[0].clone()));
                } else if param_spec.strict_default && param_spec.default.is_some() {
                    validated.push(ValidatedParam {
                        spec: param_spec.clone(),
                        value: param_spec.default.clone(),
                    });
                }
            }
        }

        // Handle unknown parameters according to policy
        self.handle_unknown_parameters(params, &handled_names, &mut validated)?;

        Ok(validated)
    }

    /// Find user input for a parameter spec (checks all aliases)
    fn find_user_input_for_param(
        &self,
        params: Option<&HashMap<String, String>>,
        param_spec: &EngineParam,
    ) -> Option<(String, String)> {
        params.and_then(|p| {
            param_spec
                .names
                .iter()
                .find_map(|name| p.get(name).map(|v| (name.clone(), v.clone())))
        })
    }

    /// Process a known parameter with full validation
    fn process_known_parameter(
        &self,
        param_spec: &EngineParam,
        name: &str,
        value_str: &str,
    ) -> Result<ValidatedParam, ValidationError> {
        // Parse string as JSON value
        let value: serde_json::Value = self.parse_parameter_value(name, value_str)?;

        // Validate type
        self.validate_type(&param_spec.param_type, &value, name)?;

        // Run all validations
        if let Some(ref validations) = param_spec.validate {
            for validation in validations {
                self.validate_value(validation, &value, name)?;
            }
        }

        Ok(ValidatedParam { spec: param_spec.clone(), value: Some(value) })
    }

    /// Parse parameter value with proper error handling
    fn parse_parameter_value(
        &self,
        _name: &str,
        value_str: &str,
    ) -> Result<serde_json::Value, ValidationError> {
        // Try parsing as JSON first
        match serde_json::from_str(value_str) {
            Ok(value) => Ok(value),
            Err(_) => {
                // If JSON parsing fails, treat as string
                // This handles cases where users pass unquoted strings
                Ok(serde_json::Value::String(value_str.to_string()))
            },
        }
    }

    /// Handle unknown parameters based on configuration policy
    fn handle_unknown_parameters(
        &self,
        params: Option<&HashMap<String, String>>,
        handled_names: &std::collections::HashSet<String>,
        validated: &mut Vec<ValidatedParam>,
    ) -> Result<(), ValidationError> {
        let Some(params) = params else {
            return Ok(());
        };

        let unknown: Vec<_> = params.keys().filter(|k| !handled_names.contains(*k)).collect();

        if unknown.is_empty() {
            return Ok(());
        }

        match self.config.engine_params.unknown_params_behavior {
            UnknownBehavior::Reject => Err(ValidationError::UnknownParameter(unknown[0].clone())),
            UnknownBehavior::Pass => {
                self.pass_unknown_parameters(params, &unknown, validated);
                Ok(())
            },
            UnknownBehavior::Strip => {
                tracing::warn!("Stripped unknown parameters: {:?}", unknown);
                Ok(())
            },
        }
    }

    /// Pass unknown parameters through as generic string parameters
    fn pass_unknown_parameters(
        &self,
        params: &HashMap<String, String>,
        unknown: &[&String],
        validated: &mut Vec<ValidatedParam>,
    ) {
        for key in unknown {
            if let Some(value_str) = params.get(*key) {
                let value = self
                    .parse_parameter_value(key, value_str)
                    .unwrap_or_else(|_| serde_json::Value::String(value_str.clone()));

                validated.push(ValidatedParam {
                    spec: EngineParam {
                        names: vec![(*key).clone()],
                        cli_flag: (*key).clone(),
                        param_type: ParamType::String,
                        default: None,
                        strict_default: false,
                        required: false,
                        sensitive: false,
                        list_separator: None,
                        boolean_style: None,
                        env_var: None,
                        validate: None,
                        description: None,
                    },
                    value: Some(value),
                });
            }
        }
    }

    /// Validate parameter type matches expected type
    fn validate_type(
        &self,
        expected: &ParamType,
        value: &serde_json::Value,
        name: &str,
    ) -> Result<(), ValidationError> {
        let matches = match (expected, value) {
            (ParamType::String, serde_json::Value::String(_)) => true,
            (ParamType::Int, serde_json::Value::Number(n)) => n.is_i64(),
            (ParamType::Float, serde_json::Value::Number(n)) => n.is_f64() || n.is_i64(),
            (ParamType::Bool, serde_json::Value::Bool(_)) => true,
            (ParamType::List, serde_json::Value::Array(_)) => true,
            // Allow string representation of numbers and booleans
            (ParamType::Int, serde_json::Value::String(s)) => s.parse::<i64>().is_ok(),
            (ParamType::Float, serde_json::Value::String(s)) => s.parse::<f64>().is_ok(),
            (ParamType::Bool, serde_json::Value::String(s)) => {
                matches!(s.to_lowercase().as_str(), "true" | "false")
            },
            _ => false,
        };

        if !matches {
            return Err(ValidationError::TypeMismatch(
                name.to_string(),
                format!("Expected {:?}, got {:?}", expected, value),
            ));
        }
        Ok(())
    }

    /// Apply specific validation rules
    fn validate_value(
        &self,
        validation: &Validation,
        value: &serde_json::Value,
        name: &str,
    ) -> Result<(), ValidationError> {
        match validation {
            Validation::Enum { allowed, message } => {
                self.validate_enum(value, allowed, message, name)
            },
            Validation::Regex { pattern, message } => {
                self.validate_regex(value, pattern, message, name)
            },
            Validation::Range { min, max, message } => {
                self.validate_range(value, *min, *max, message, name)
            },
            Validation::Custom { rule, message } => {
                self.validate_custom_rule(rule, value, name, message)
            },
            Validation::FileExists { message } => self.validate_file_exists(value, message, name),
            Validation::FileExtension { allowed, message } => {
                self.validate_file_extension(value, allowed, message, name)
            },
        }
    }

    /// Validate enum values
    fn validate_enum(
        &self,
        value: &serde_json::Value,
        allowed: &[String],
        message: &Option<String>,
        name: &str,
    ) -> Result<(), ValidationError> {
        match value {
            serde_json::Value::String(s) => {
                if !allowed.contains(s) {
                    return Err(ValidationError::ValidationFailed(
                        name.to_string(),
                        message
                            .clone()
                            .unwrap_or_else(|| format!("Must be one of: {}", allowed.join(", "))),
                    ));
                }
            },
            serde_json::Value::Array(arr) => {
                for item in arr {
                    if let Some(s) = item.as_str()
                        && !allowed.contains(&s.to_string())
                    {
                        return Err(ValidationError::ValidationFailed(
                            name.to_string(),
                            message.clone().unwrap_or_else(|| {
                                format!("All items must be one of: {}", allowed.join(", "))
                            }),
                        ));
                    }
                }
            },
            _ => {
                return Err(ValidationError::ValidationFailed(
                    name.to_string(),
                    "Expected string or array for enum validation".to_string(),
                ));
            },
        }
        Ok(())
    }

    /// Validate regex patterns
    fn validate_regex(
        &self,
        value: &serde_json::Value,
        pattern: &str,
        message: &Option<String>,
        name: &str,
    ) -> Result<(), ValidationError> {
        let val_str = value.as_str().ok_or_else(|| {
            ValidationError::ValidationFailed(
                name.to_string(),
                "Expected string for regex validation".to_string(),
            )
        })?;

        let re = Regex::new(pattern).map_err(|e| {
            ValidationError::ValidationFailed(
                name.to_string(),
                format!("Invalid regex pattern in config: {}", e),
            )
        })?;

        if !re.is_match(val_str) {
            return Err(ValidationError::ValidationFailed(
                name.to_string(),
                message
                    .clone()
                    .unwrap_or_else(|| format!("Value '{}' does not match pattern", val_str)),
            ));
        }
        Ok(())
    }

    /// Validate numeric ranges
    fn validate_range(
        &self,
        value: &serde_json::Value,
        min: Option<i64>,
        max: Option<i64>,
        message: &Option<String>,
        name: &str,
    ) -> Result<(), ValidationError> {
        let val_int = value
            .as_i64()
            .or_else(|| value.as_str().and_then(|s| s.parse().ok()))
            .ok_or_else(|| {
                ValidationError::ValidationFailed(
                    name.to_string(),
                    "Expected integer for range validation".to_string(),
                )
            })?;

        if let Some(min_val) = min
            && val_int < min_val
        {
            return Err(ValidationError::ValidationFailed(
                name.to_string(),
                message.clone().unwrap_or_else(|| format!("Must be >= {}", min_val)),
            ));
        }

        if let Some(max_val) = max
            && val_int > max_val
        {
            return Err(ValidationError::ValidationFailed(
                name.to_string(),
                message.clone().unwrap_or_else(|| format!("Must be <= {}", max_val)),
            ));
        }
        Ok(())
    }

    /// Validate file existence
    fn validate_file_exists(
        &self,
        value: &serde_json::Value,
        message: &Option<String>,
        name: &str,
    ) -> Result<(), ValidationError> {
        let path = value.as_str().ok_or_else(|| {
            ValidationError::ValidationFailed(
                name.to_string(),
                "Expected string path for file validation".to_string(),
            )
        })?;

        if !std::path::Path::new(path).exists() {
            return Err(ValidationError::ValidationFailed(
                name.to_string(),
                message.clone().unwrap_or_else(|| format!("File '{}' does not exist", path)),
            ));
        }
        Ok(())
    }

    /// Validate file extensions
    fn validate_file_extension(
        &self,
        value: &serde_json::Value,
        allowed: &[String],
        message: &Option<String>,
        name: &str,
    ) -> Result<(), ValidationError> {
        let path = value.as_str().ok_or_else(|| {
            ValidationError::ValidationFailed(name.to_string(), "Expected string path".to_string())
        })?;

        let ext = std::path::Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| format!(".{}", e));

        if let Some(ext) = ext {
            if !allowed.contains(&ext) {
                return Err(ValidationError::ValidationFailed(
                    name.to_string(),
                    message.clone().unwrap_or_else(|| {
                        format!("File extension must be one of: {}", allowed.join(", "))
                    }),
                ));
            }
        } else {
            return Err(ValidationError::ValidationFailed(
                name.to_string(),
                message.clone().unwrap_or_else(|| {
                    format!("File must have one of these extensions: {}", allowed.join(", "))
                }),
            ));
        }
        Ok(())
    }

    /// Handle custom validation rules
    fn validate_custom_rule(
        &self,
        rule: &str,
        value: &serde_json::Value,
        name: &str,
        message: &Option<String>,
    ) -> Result<(), ValidationError> {
        match rule {
            "no_parent_traversal" => {
                let val_str = value.as_str().ok_or_else(|| {
                    ValidationError::ValidationFailed(
                        name.to_string(),
                        "Expected string for path validation".to_string(),
                    )
                })?;

                if val_str.contains("..") || val_str.contains("/../") || val_str.starts_with("../")
                {
                    return Err(ValidationError::ValidationFailed(
                        name.to_string(),
                        message
                            .clone()
                            .unwrap_or_else(|| "Path traversal (..) is not allowed".to_string()),
                    ));
                }
            },
            "writable_directory" => {
                let val_str = value.as_str().ok_or_else(|| {
                    ValidationError::ValidationFailed(
                        name.to_string(),
                        "Expected string for directory validation".to_string(),
                    )
                })?;

                let path = std::path::Path::new(val_str);
                if path.exists() && !path.is_dir() {
                    return Err(ValidationError::ValidationFailed(
                        name.to_string(),
                        message.clone().unwrap_or_else(|| "Path is not a directory".to_string()),
                    ));
                    // TODO: Check if directory is actually writable
                    // This would require testing write permissions which may
                    // not be desired in validation phase
                }
            },
            "absolute_path" => {
                let val_str = value.as_str().ok_or_else(|| {
                    ValidationError::ValidationFailed(
                        name.to_string(),
                        "Expected string for path validation".to_string(),
                    )
                })?;

                if !std::path::Path::new(val_str).is_absolute() {
                    return Err(ValidationError::ValidationFailed(
                        name.to_string(),
                        message.clone().unwrap_or_else(|| "Path must be absolute".to_string()),
                    ));
                }
            },
            "safe_filename" => {
                let val_str = value.as_str().ok_or_else(|| {
                    ValidationError::ValidationFailed(
                        name.to_string(),
                        "Expected string for filename validation".to_string(),
                    )
                })?;

                // Check for unsafe characters in filenames
                let unsafe_chars = ['/', '\\', ':', '*', '?', '"', '<', '>', '|'];
                if unsafe_chars.iter().any(|&c| val_str.contains(c)) {
                    return Err(ValidationError::ValidationFailed(
                        name.to_string(),
                        message
                            .clone()
                            .unwrap_or_else(|| "Filename contains unsafe characters".to_string()),
                    ));
                }
            },
            _ => {
                tracing::warn!("Unknown custom validation rule: {}", rule);
            },
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::configs::*;

    fn create_test_config() -> EngineConfig {
        EngineConfig {
            name: "test".to_string(),
            id: uuid::Uuid::now_v7(),
            version: "1.0".to_string(),
            workflow_types: vec!["test".to_string()],
            workflow_type_versions: vec!["1.0".to_string()],
            command_template: "test {engine_params}".to_string(),
            backend: Backend::Local,
            workflow_params: WorkflowParamsConfig {
                style: WorkflowParamsStyle {
                    method: WorkflowParamsMethod::Inline,
                    prefix: Some("--".to_string()),
                    separator: Some("=".to_string()),
                    key_value_format: None,
                    format: None,
                    file_path: None,
                    params_file_flag: None,
                },
            },
            engine_params: EngineParamsConfig {
                unknown_params_behavior: UnknownBehavior::Reject,
                validated_params: vec![],
            },
            denied_params: vec![],
            ignored_params: None,
        }
    }

    #[test]
    fn test_validator_creation() {
        let config = create_test_config();
        let validator = EngineRequestValidator::new(config);
        assert_eq!(validator.config.name, "test");
    }

    #[test]
    fn test_empty_request_validation() {
        let config = create_test_config();
        let validator = EngineRequestValidator::new(config.clone());

        let request = RunRequest {
            workflow_params: None,
            workflow_type: "test".to_string(),
            workflow_type_version: "1.0".to_string(),
            tags: None,
            workflow_engine_parameters: None,
            workflow_engine: "test".to_string(),
            workflow_engine_version: "1.0".to_string(),
            workflow_url: "http://example.com/workflow".to_string(),
        };

        let result = validator.validate(&request);
        assert!(result.is_ok());
    }
}
