use crate::models::run_context::RunContext;
use crate::models::test_result::TestResult;
use crate::pipeline::templating::liquid::LiquidEngine;
use crate::pipeline::templating::TemplateEngine;
use crate::pipeline::variables::VariableAssignment;

pub struct DefaultVariableAssignment{
    var: String,
    liquid: LiquidEngine,
}
impl DefaultVariableAssignment{
    pub fn new(var: &str) -> Self{
        Self{
            var: var.to_string(),
            liquid: LiquidEngine
        }
    }
}
impl VariableAssignment for DefaultVariableAssignment {
    fn set(&self, data: &TestResult, context: &mut RunContext) {
        if let Some((key, value)) = self.var.split_once('=') {
            let value = self.liquid.render(value, &data.to_liquid_template()).unwrap_or_else(|e| e.to_string());
            context.file.insert(key.to_string(), value.to_string());
        }
    }
}