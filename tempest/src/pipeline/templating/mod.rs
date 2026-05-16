pub mod liquid;
pub mod liquid_filters;

pub trait TemplateEngine {
    fn render(&self, source: &str, context: &liquid_core::Object) -> anyhow::Result<String>;
}