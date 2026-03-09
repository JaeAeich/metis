use std::collections::HashMap;

use common::models::ValidatedRunRequest;
use uuid::Uuid;

use crate::models::CommandInfo;

pub struct DryRunReport {
    pub run_id: Uuid,
    pub user_id: String,
    pub workflow_url: String,
    pub workflow_type: String,
    pub workflow_type_version: String,
    pub workdir: String,
    pub subdirs: HashMap<String, String>,
    pub command_info: CommandInfo,
    pub request: ValidatedRunRequest,
}

impl DryRunReport {
    pub fn print(&self) {
        println!("DRY RUN SUMMARY");
        println!("  run_id: {}", self.run_id);
        println!("  user_id: {}", self.user_id);
        println!("  workflow_url: {}", self.workflow_url);
        println!("  workflow_type: {} {}", self.workflow_type, self.workflow_type_version);
        println!();

        println!("  staging_area:");
        println!("    workdir: {}", self.workdir);
        for (name, path) in &self.subdirs {
            println!("    {}: {}", name, path);
        }
        println!();

        println!("  command:");
        let cmd = &self.command_info.command;
        println!("    cd {} && {}", self.command_info.workdir, cmd);
        println!();

        if !self.command_info.env_vars.is_empty() {
            println!("  environment_variables:");
            for (k, v) in &self.command_info.env_vars {
                let is_sensitive = self
                    .request
                    .workflow_engine_parameters
                    .as_ref()
                    .map(|params| {
                        params
                            .iter()
                            .any(|p| p.spec.env_var.as_deref() == Some(k) && p.spec.sensitive)
                    })
                    .unwrap_or(false);

                if is_sensitive {
                    println!("    {}: [REDACTED]", k);
                } else {
                    println!("    {}: {}", k, v);
                }
            }
            println!();
        }

        if let Some(ref params) = self.request.workflow_params {
            println!("  workflow_parameters:");
            if let Some(obj) = params.as_object() {
                for (k, v) in obj {
                    println!("    {}: {}", k, v);
                }
            }
            println!();
        }

        if let Some(ref params) = self.request.workflow_engine_parameters {
            println!("  engine_parameters:");
            for p in params {
                let value_str = p
                    .value
                    .as_ref()
                    .map(std::string::ToString::to_string)
                    .unwrap_or_else(|| "(default)".to_string());
                let sensitive = if p.spec.sensitive { " [SENSITIVE]" } else { "" };
                let name = p.spec.names.first().map(String::as_str).unwrap_or("");
                println!("    {}: {}{}", name, value_str, sensitive);
            }
        }
    }
}
