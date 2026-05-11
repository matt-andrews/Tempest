use async_trait::async_trait;
use crate::engine::runner::RunnerCapability;
use crate::models::descriptor_model::DescriptorModel;
use crate::models::run_result::{Assertion, RunResult};
use colored::{ColoredString, Colorize};

pub struct ConsoleReportCapability;

impl ConsoleReportCapability {
    fn pass(){
        println!("   {}", "PASSED".bright_green());
    }
    fn fail(){
        println!("   {}", "FAILED".bright_red());
    }
    fn render_assertions(assertions: &[Assertion]) -> bool {
        assertions.iter().fold(true, |passed, assert| {
            let (icon, expr) = if assert.passed {
                ("✅", assert.expr.green())
            } else {
                ("❌", assert.expr.red())
            };
            println!("       {icon}  {expr}");
            if !assert.passed && !assert.error.is_empty() {
                println!("          {}", assert.error.red());
            }
            passed && assert.passed
        })
    }
    fn color_status(status: u16) -> ColoredString{
        match status {
            200..=299 => status.to_string().green(),
            300..=399 => status.to_string().yellow(),
            400..=499 => status.to_string().red(),
            _ => status.to_string().normal(),
        }
    }
    fn color_duration(duration: &core::time::Duration) -> ColoredString{
        let duration_str = format!("{:.3}ms", duration.as_secs_f64() * 1000.0);
        match duration.as_millis() {
            0..=50   => duration_str.green(),
            51..=200 => duration_str.yellow(),
            _        => duration_str.red(),
        }
    }
}
#[async_trait]
impl RunnerCapability for ConsoleReportCapability {
    async fn run(&self, descriptor: &DescriptorModel, context: Option<RunResult>) -> RunResult {
        let name = descriptor.name.clone().unwrap_or_default();

        if descriptor.test.is_none() {
            println!("\n{}:", name.bright_blue());
            return context.unwrap_or_default();
        }

        let Some(ctx) = &context else {
            Self::fail();
            return RunResult::default();
        };
        let Some(http_result) = ctx.success.then(|| ctx.http_result.as_ref()).flatten() else {
            Self::fail();
            return RunResult::default();
        };

        let status = Self::color_status(http_result.status.as_u16());
        let duration = Self::color_duration(&http_result.duration);
        println!(" - {}  {}  {}", name.bright_purple(), status, duration);

        let passed = Self::render_assertions(ctx.assertions.as_deref().unwrap_or_default());
        if passed {
            Self::pass();
        } else {
            Self::fail();
            match serde_json::to_string_pretty(ctx) {
                Ok(json) => println!("{json}"),
                Err(e)   => println!("(serialization error: {e})"),
            }
        }

        context.unwrap_or_default()
    }
}