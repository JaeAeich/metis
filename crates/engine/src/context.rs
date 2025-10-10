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
        vars.insert(
            "date".to_string(),
            self.timestamp.format("%Y-%m-%d").to_string(),
        );
        vars.insert(
            "time".to_string(),
            self.timestamp.format("%H-%M-%S").to_string(),
        );

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
