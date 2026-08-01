use std::collections::HashMap;

pub mod liquid;
pub mod liquid_filters;

pub trait TemplateEngine {
    fn render(&self, source: &str, context: &liquid_core::Object) -> anyhow::Result<String>;
    fn render_string_or_self(&self, source: &str, context: &liquid_core::Object) -> String;
    fn render_option_string_or_self(
        &self,
        source: &Option<String>,
        context: &liquid_core::Object,
    ) -> Option<String>;
    fn render_vec_string_or_self(
        &self,
        source: &Option<Vec<String>>,
        context: &liquid_core::Object,
    ) -> Option<Vec<String>>;
    fn render_hashmap_string_or_self(
        &self,
        source: &Option<HashMap<String, String>>,
        context: &liquid_core::Object,
    ) -> Option<HashMap<String, String>>;
}
