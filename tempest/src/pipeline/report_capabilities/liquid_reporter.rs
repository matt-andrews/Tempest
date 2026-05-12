use std::collections::HashMap;
use std::sync::LazyLock;
use liquid::model::Value;
use crate::models::descriptor_model::DescriptorModel;
use crate::models::options_model::OptionsModel;
use crate::models::report_template_model::ReportTemplateModel;
use crate::models::test_result::{Assertion, TestResult};
use crate::pipeline::report_capabilities::ReportCapability;

static PARSER: LazyLock<liquid::Parser> = LazyLock::new(|| {
    liquid::ParserBuilder::with_stdlib()
        .build()
        .expect("failed to build Liquid parser")
});

pub struct LiquidReporter;

impl ReportCapability for LiquidReporter {
    fn report(
        &self,
        descriptor: &DescriptorModel,
        test_result: Option<TestResult>,
        assertions: Vec<Assertion>,
        options: OptionsModel,
        templates: &HashMap<String, ReportTemplateModel>,
    ) {
        let report_names = options.reports.unwrap_or_default();
        let active: Vec<&ReportTemplateModel> = templates
            .iter()
            .filter(|(key, _)| report_names.contains(key))
            .map(|(_, v)| v)
            .collect();

        if active.is_empty() {
            return;
        }

        let globals = build_globals(descriptor, test_result.as_ref(), &assertions);

        println!();
        for template in active {
            if descriptor.test.is_none() {
                match PARSER.parse(&template.section.clone().unwrap_or_default()) {
                    Ok(tmpl) => match tmpl.render(&globals) {
                        Ok(output) => print!("{output}"),
                        Err(e) => eprintln!("liquid render error: {e}"),
                    },
                    Err(e) => eprintln!("liquid parse error: {e}"),
                }
            }else {
                match PARSER.parse(&template.test.clone().unwrap_or_default()) {
                    Ok(tmpl) => match tmpl.render(&globals) {
                        Ok(output) => print!("{output}"),
                        Err(e) => eprintln!("liquid render error: {e}"),
                    },
                    Err(e) => eprintln!("liquid parse error: {e}"),
                }
            }
        }
    }
}

fn build_globals(
    descriptor: &DescriptorModel,
    test_result: Option<&TestResult>,
    assertions: &[Assertion],
) -> liquid::Object {
    let all_passed = assertions.iter().all(|a| a.passed);

    let assertion_values: Vec<Value> = assertions
        .iter()
        .map(|a| Value::Object(liquid::object!({
            "expr":   a.expr.clone(),
            "passed": a.passed,
            "error":  a.error.clone(),
        })))
        .collect();

    let mut globals = if let Some(result) = test_result {
        liquid::object!({
            "name":           descriptor.name.clone().unwrap_or_default(),
            "description":    descriptor.description.clone().unwrap_or_default(),
            "passed":         all_passed,
            "status":         result.status.code as i64,
            "status_message": result.status.message.clone(),
            "body":           result.body.clone(),
            "duration":    result.duration.as_secs_f64() * 1000.0,
        })
    } else {
        liquid::object!({
            "name":        descriptor.name.clone().unwrap_or_default(),
            "description": descriptor.description.clone().unwrap_or_default(),
            "passed":      all_passed,
        })
    };

    globals.insert("assertions".into(), Value::Array(assertion_values));
    globals
}
