use crate::models::run_options::RunOptions;
use crate::templating::TemplateEngine;
use crate::templating::liquid::LiquidEngine;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct Project {
    pub name: String,
    pub version: Option<String>,
    pub options: Option<RunOptions>,
    pub env: Option<HashMap<String, String>>,
    pub include: Option<Vec<PathBuf>>,

    pub warn_as_err: Option<bool>,
    pub success_exit: Option<i32>,
    pub flaky_exit: Option<i32>,
    pub failed_exit: Option<i32>,
}

impl Project {
    pub fn new_with_defaults() -> Self {
        Self {
            success_exit: Some(0),
            flaky_exit: Some(0),
            failed_exit: Some(1),
            ..Project::default()
        }
    }
    pub fn merge_env(&mut self, parent_env: HashMap<String, String>) {
        let mut parent_env = parent_env;
        if let Some(env) = &self.env {
            for (k, v) in env {
                parent_env.insert(k.clone(), v.clone());
            }
        }
        self.env = Some(parent_env);
    }
    pub fn render_template(&mut self, engine: &LiquidEngine) {
        if let Some(env) = &self.env {
            let obj = &liquid::object!({
                "env": &env,
            });
            self.name = engine.render_string_or_self(&self.name, obj);
            self.version = engine.render_option_string_or_self(&self.version, obj);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_with_defaults_sets_expected_exit_codes() {
        let project = Project::new_with_defaults();

        assert_eq!(project.success_exit, Some(0));
        assert_eq!(project.flaky_exit, Some(0));
        assert_eq!(project.failed_exit, Some(1));
    }

    #[test]
    fn merge_env_preserves_parent_values_and_prefers_project_values() {
        let mut project = Project {
            env: Some(HashMap::from([
                ("SHARED".to_owned(), "project".to_owned()),
                ("PROJECT_ONLY".to_owned(), "project-value".to_owned()),
            ])),
            ..Project::default()
        };
        let parent = HashMap::from([
            ("SHARED".to_owned(), "parent".to_owned()),
            ("PARENT_ONLY".to_owned(), "parent-value".to_owned()),
        ]);

        project.merge_env(parent);

        let env = project.env.unwrap();
        assert_eq!(env["SHARED"], "project");
        assert_eq!(env["PROJECT_ONLY"], "project-value");
        assert_eq!(env["PARENT_ONLY"], "parent-value");
    }

    #[test]
    fn render_template_renders_name_and_version_from_environment() {
        let mut project = Project {
            name: "{{ env.NAME | upcase }} service".to_owned(),
            version: Some("{{ env.VERSION }}".to_owned()),
            env: Some(HashMap::from([
                ("NAME".to_owned(), "tempest".to_owned()),
                ("VERSION".to_owned(), "2.0".to_owned()),
            ])),
            ..Project::default()
        };

        project.render_template(&LiquidEngine);

        assert_eq!(project.name, "TEMPEST service");
        assert_eq!(project.version.as_deref(), Some("2.0"));
    }
}
