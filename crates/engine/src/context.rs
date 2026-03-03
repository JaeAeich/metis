use crate::models::BuildContext;
use std::collections::HashMap;

impl BuildContext {
    pub fn template_vars(&self) -> HashMap<String, String> {
        let mut vars = HashMap::new();
        vars.insert("run_id".to_string(), self.run_id.clone());
        vars.insert("user_id".to_string(), self.user_id.clone());
        vars.insert("workflow_path".to_string(), self.workflow_path.clone());
        vars.insert("workflow_url".to_string(), self.workflow_url.clone());
        vars.insert("workdir".to_string(), self.workdir.clone());
        vars.insert("timestamp".to_string(), self.timestamp.to_rfc3339());
        vars.insert("params_file".to_string(), self.params_file.clone());
        vars.insert("workflow_params".to_string(), self.workflow_params.clone());
        vars.insert("engine_params".to_string(), self.engine_params.clone());
        vars.insert("date".to_string(), self.date.clone());
        vars.insert("time".to_string(), self.time.clone());
        vars.insert("logs_dir".to_string(), self.log_dir.clone());
        vars.insert("outputs_dir".to_string(), self.output_dir.clone());

        for (name, path) in &self.subdirs {
            vars.insert(format!("{}_dir", name), path.clone());
        }

        vars
    }

    pub fn replace_vars(&self, template: &str) -> String {
        let vars = self.template_vars();
        let mut result = template.to_string();

        for (key, value) in vars {
            result = result.replace(&format!("{{{}}}", key), &value);
        }

        result
    }
}
