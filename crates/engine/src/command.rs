use crate::models::{BuildContext, CommandInfo};
use common::configs::{EngineConfig, EngineParam, ParamType};
use common::models::ValidatedParam;
use common::models::ValidatedRunRequest;
use std::collections::HashMap;
use std::sync::Arc;

pub struct EngineCommandBuilder {
    config: Arc<EngineConfig>,
}

impl EngineCommandBuilder {
    pub fn new(config: Arc<EngineConfig>) -> Self {
        Self { config }
    }

    fn config(&self) -> &EngineConfig {
        &self.config
    }

    pub fn build_command(
        &self,
        request: &ValidatedRunRequest,
        context: &BuildContext,
    ) -> Result<CommandInfo, Box<dyn std::error::Error + Send + Sync>> {
        let validated_params = request.workflow_engine_parameters.as_ref();

        let env_vars = self.collect_env_vars(validated_params, context);

        let mut template_vars = context.template_vars();
        template_vars.insert("engine_params".to_string(), context.engine_params.clone());
        template_vars.insert(
            "workflow_params".to_string(),
            context.workflow_params.clone(),
        );

        let command = self.replace_template_vars(&self.config().command_template, &template_vars);

        let redacted_command = self.redact_sensitive(&command, validated_params);

        let full_command = self.build_full_command(&command, &env_vars);
        let full_redacted_command = self.build_full_command(&redacted_command, &env_vars);

        Ok(CommandInfo {
            command: full_command,
            redacted_command: full_redacted_command,
            env_vars,
            workdir: context.workdir.clone(),
        })
    }

    pub fn build_engine_params_string(
        &self,
        params: Option<&Vec<ValidatedParam>>,
        context: &BuildContext,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        self.build_engine_params_string_internal(params, context)
    }

    pub fn build_workflow_params_string(
        &self,
        params: Option<&serde_json::Value>,
        context: &BuildContext,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        self.build_workflow_params_string_internal(params, context)
    }

    fn build_engine_params_string_internal(
        &self,
        params: Option<&Vec<ValidatedParam>>,
        context: &BuildContext,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let Some(params) = params else {
            return Ok(String::new());
        };

        let mut parts = Vec::new();

        for param in params {
            if param.spec.env_var.is_some() {
                continue;
            }

            if let Some(ref value) = param.value {
                let param_str = match param.spec.param_type {
                    ParamType::Bool => self.format_bool_param(&param.spec, value)?,
                    ParamType::List => self.format_list_param(&param.spec, value)?,
                    ParamType::String => {
                        let val_str = value.as_str().ok_or("Expected string")?;
                        let expanded = context.replace_vars(val_str);
                        format!("{} {}", param.spec.cli_flag, expanded)
                    }
                    _ => format!("{} {}", param.spec.cli_flag, value),
                };

                if !param_str.is_empty() {
                    parts.push(param_str);
                }
            }
        }

        Ok(parts.join(" "))
    }

    fn format_bool_param(
        &self,
        spec: &EngineParam,
        value: &serde_json::Value,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let bool_val = value
            .as_bool()
            .or_else(|| {
                value
                    .as_str()
                    .and_then(|v| match v.to_ascii_lowercase().as_str() {
                        "true" => Some(true),
                        "false" => Some(false),
                        _ => None,
                    })
            })
            .unwrap_or(false);
        if !bool_val {
            return Ok(String::new());
        }

        let style = spec
            .boolean_style
            .as_ref()
            .unwrap_or(&common::configs::BooleanStyle::Flag);
        Ok(match style {
            common::configs::BooleanStyle::Flag => spec.cli_flag.clone(),
            common::configs::BooleanStyle::Value => format!("{} true", spec.cli_flag),
            common::configs::BooleanStyle::Equals => format!("{}=true", spec.cli_flag),
        })
    }

    fn format_list_param(
        &self,
        spec: &EngineParam,
        value: &serde_json::Value,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let list = value.as_array().ok_or("Expected array")?;
        let items: Vec<String> = list
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect();

        let separator = spec.list_separator.as_deref().unwrap_or(",");

        Ok(format!("{} {}", spec.cli_flag, items.join(separator)))
    }

    fn build_workflow_params_string_internal(
        &self,
        params: Option<&serde_json::Value>,
        context: &BuildContext,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let Some(params) = params else {
            return Ok(String::new());
        };

        let params_map = params
            .as_object()
            .ok_or("workflow_params must be an object")?;
        let style = &self.config().workflow_params.style;

        match style.method {
            common::configs::WorkflowParamsMethod::Inline => {
                let prefix = style.prefix.as_deref().unwrap_or("--");
                let separator = style.separator.as_deref().unwrap_or(" ");

                let param_strings: Vec<String> = params_map
                    .iter()
                    .map(|(k, v)| {
                        let val = match v {
                            serde_json::Value::String(s) => s.clone(),
                            _ => {
                                let s = v.to_string();
                                s.trim_matches('"').to_string()
                            }
                        };
                        format!("{}{}{}{}", prefix, k, separator, val)
                    })
                    .collect();

                Ok(param_strings.join(" "))
            }
            common::configs::WorkflowParamsMethod::File => {
                let file_path = style
                    .file_path
                    .as_deref()
                    .unwrap_or("{workdir}/workflow-params.json");
                let expanded_path = context.replace_vars(file_path);
                Ok(format!("--params-file {}", expanded_path))
            }
        }
    }

    fn collect_env_vars(
        &self,
        params: Option<&Vec<ValidatedParam>>,
        _context: &BuildContext,
    ) -> HashMap<String, String> {
        let mut env_vars = HashMap::new();

        if let Some(params) = params {
            for param in params {
                if let (Some(env_var), Some(value)) = (&param.spec.env_var, &param.value)
                    && let Some(val_str) = value.as_str()
                {
                    env_vars.insert(env_var.clone(), val_str.to_string());
                }
            }
        }

        env_vars
    }

    fn replace_template_vars(&self, template: &str, vars: &HashMap<String, String>) -> String {
        let mut result = template.to_string();
        for (key, value) in vars {
            result = result.replace(&format!("{{{}}}", key), value);
        }
        result
    }

    fn build_full_command(&self, command: &str, env_vars: &HashMap<String, String>) -> String {
        if env_vars.is_empty() {
            return command.to_string();
        }

        let env_prefix: String = env_vars
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join(" ");

        format!("{} {}", env_prefix, command)
    }

    fn redact_sensitive(&self, command: &str, params: Option<&Vec<ValidatedParam>>) -> String {
        let Some(params) = params else {
            return command.to_string();
        };

        let mut redacted = command.to_string();
        for param in params {
            if param.spec.sensitive
                && let Some(ref value) = param.value
                && let Some(val_str) = value.as_str()
            {
                redacted = redacted.replace(val_str, "[REDACTED]");
            }
        }
        redacted
    }
}
