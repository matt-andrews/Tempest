use crate::models::descriptor::Profile;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IterationContext {
    pub case_index: usize,
    pub case_count: usize,
    pub has_profile: bool,
    pub profile_index: usize,
    pub profile_count: usize,
    pub has_loop: bool,
    pub loop_index: usize,
    pub loop_count: usize,
}

pub struct IterationScope {
    previous_profile: Profile,
    pushed_profile: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunContext {
    pub file_name: String,
    pub vars: HashMap<String, JsonValue>,
    pub env: HashMap<String, String>,
    pub retry_attempts: usize,
    pub profile: Profile,
    pub profile_stack: Vec<Profile>,
    pub iteration: Option<IterationContext>,
    pub iteration_stack: Vec<IterationContext>,

    #[serde(skip)]
    collected_vars: HashSet<String>,
}

impl RunContext {
    pub fn new(file_name: &str, env: &HashMap<String, String>) -> Self {
        Self {
            file_name: file_name.to_owned(),
            vars: HashMap::new(),
            env: env.to_owned(),
            retry_attempts: 0,
            profile: Profile::new(),
            profile_stack: Vec::new(),
            iteration: None,
            iteration_stack: Vec::new(),
            collected_vars: HashSet::new(),
        }
    }

    pub fn enter_iteration(
        &mut self,
        profile: Option<Profile>,
        iteration: IterationContext,
    ) -> IterationScope {
        let previous_profile = self.profile.clone();
        let pushed_profile = profile.is_some();

        if let Some(profile) = profile {
            self.profile.extend(profile.clone());
            self.profile_stack.push(profile);
        }

        self.iteration_stack.push(iteration.clone());
        self.iteration = Some(iteration);

        IterationScope {
            previous_profile,
            pushed_profile,
        }
    }

    pub fn exit_iteration(&mut self, scope: IterationScope) {
        self.iteration_stack.pop();
        self.iteration = self.iteration_stack.last().cloned();
        self.profile = scope.previous_profile;
        if scope.pushed_profile {
            self.profile_stack.pop();
        }
    }

    pub fn is_expanded(&self) -> bool {
        !self.iteration_stack.is_empty()
    }

    pub fn expansion_prefix(&self) -> String {
        self.iteration_stack
            .iter()
            .flat_map(|iteration| {
                let mut locations = Vec::with_capacity(2);
                if iteration.has_profile {
                    locations.push(format!(
                        "[profile #{}/{}]",
                        iteration.profile_index + 1,
                        iteration.profile_count
                    ));
                }
                if iteration.has_loop {
                    locations.push(format!(
                        "[loop #{}/{}]",
                        iteration.loop_index + 1,
                        iteration.loop_count
                    ));
                }
                locations
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    pub fn set_var(&mut self, name: String, value: JsonValue) {
        if !self.is_expanded() && !self.collected_vars.contains(&name) {
            self.vars.insert(name, value);
            return;
        }

        if self.collected_vars.insert(name.clone()) {
            let mut values = self
                .vars
                .remove(&name)
                .map_or_else(Vec::new, |value| vec![value]);
            values.push(value);
            self.vars.insert(name, JsonValue::Array(values));
            return;
        }

        match self.vars.entry(name) {
            std::collections::hash_map::Entry::Occupied(mut entry) => {
                if let JsonValue::Array(values) = entry.get_mut() {
                    values.push(value);
                } else {
                    let previous = std::mem::replace(entry.get_mut(), JsonValue::Null);
                    *entry.get_mut() = JsonValue::Array(vec![previous, value]);
                }
            }
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(JsonValue::Array(vec![value]));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn iteration() -> IterationContext {
        IterationContext {
            case_index: 0,
            case_count: 2,
            has_profile: false,
            profile_index: 0,
            profile_count: 1,
            has_loop: true,
            loop_index: 0,
            loop_count: 2,
        }
    }

    #[test]
    fn expanded_variables_are_collected_and_continue_appending_after_scope_exit() {
        let mut context = RunContext::new("test.yml", &HashMap::new());
        let scope = context.enter_iteration(None, iteration());
        context.set_var("id".to_string(), json!(1));
        context.set_var("id".to_string(), json!(2));
        context.exit_iteration(scope);
        context.set_var("id".to_string(), json!(3));

        assert_eq!(context.vars["id"], json!([1, 2, 3]));
    }

    #[test]
    fn scalar_is_promoted_without_flattening_array_values() {
        let mut context = RunContext::new("test.yml", &HashMap::new());
        context.set_var("items".to_string(), json!([1, 2]));
        let scope = context.enter_iteration(None, iteration());
        context.set_var("items".to_string(), json!([3, 4]));
        context.exit_iteration(scope);

        assert_eq!(context.vars["items"], json!([[1, 2], [3, 4]]));
    }

    #[test]
    fn profile_overrides_are_restored_without_restoring_variables() {
        let mut context = RunContext::new("test.yml", &HashMap::new());
        context.profile.insert("region".to_string(), json!("us"));
        let scope = context.enter_iteration(
            Some(Profile::from_iter([
                ("region".to_string(), json!("eu")),
                ("role".to_string(), json!("admin")),
            ])),
            iteration(),
        );
        context.set_var("id".to_string(), json!(42));

        assert_eq!(context.profile["region"], json!("eu"));
        assert_eq!(context.profile["role"], json!("admin"));

        context.exit_iteration(scope);

        assert_eq!(
            context.profile,
            Profile::from_iter([("region".to_string(), json!("us"))])
        );
        assert_eq!(context.vars["id"], json!([42]));
    }

    #[test]
    fn expansion_prefix_describes_every_active_profile_and_loop() {
        let mut context = RunContext::new("test.yml", &HashMap::new());
        let outer = context.enter_iteration(
            None,
            IterationContext {
                case_index: 1,
                case_count: 6,
                has_profile: true,
                profile_index: 0,
                profile_count: 2,
                has_loop: true,
                loop_index: 1,
                loop_count: 3,
            },
        );
        let inner = context.enter_iteration(
            None,
            IterationContext {
                case_index: 1,
                case_count: 2,
                has_profile: true,
                profile_index: 1,
                profile_count: 2,
                has_loop: false,
                loop_index: 0,
                loop_count: 1,
            },
        );

        assert_eq!(
            context.expansion_prefix(),
            "[profile #1/2] [loop #2/3] [profile #2/2]"
        );

        context.exit_iteration(inner);
        context.exit_iteration(outer);
        assert!(context.expansion_prefix().is_empty());
    }
}
