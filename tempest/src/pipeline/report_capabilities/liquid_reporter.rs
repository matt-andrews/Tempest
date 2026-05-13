use std::collections::HashMap;
use std::sync::LazyLock;
use liquid::model::Value;
use crate::models::descriptor_model::DescriptorModel;
use crate::models::options_model::OptionsModel;
use crate::models::report_template_model::ReportTemplateModel;
use crate::models::summary_result::SummaryResult;
use crate::models::test_result::{Assertion, TestResult};
use crate::pipeline::report_capabilities::ReportCapability;

static PARSER: LazyLock<liquid::Parser> = LazyLock::new(|| {
    use crate::utils::liquid_filters::*;
    liquid::ParserBuilder::with_stdlib()
        .filter(RedFilter)
        .filter(GreenFilter)
        .filter(YellowFilter)
        .filter(BrightRedFilter)
        .filter(BrightGreenFilter)
        .filter(BrightBlueFilter)
        .filter(BrightPurpleFilter)
        .filter(ColorStatusFilter)
        .filter(ColorDurationFilter)
        .build()
        .expect("failed to build Liquid parser")
});

pub struct LiquidReporter;

impl ReportCapability for LiquidReporter {
    fn report(
        &self,
        descriptor: &DescriptorModel,
        test_result: Option<&TestResult>,
        assertions: &[Assertion],
        options: &OptionsModel,
        templates: &HashMap<String, ReportTemplateModel>,
    ) {
        let active = build_template_iterator(options, templates);

        if active.is_empty() {
            return;
        }


        for template in active {
            if descriptor.test.is_none() {
                print(
                    descriptor,
                    test_result,
                    template,
                    assertions,
                    &template.section_template.clone().unwrap_or_default()
                );
            }else {
                print(
                    descriptor,
                    test_result,
                    template,
                    assertions,
                    &template.test_template.clone().unwrap_or_default()
                );
            }
        }
    }

    fn summary(
        &self,
        options: &OptionsModel,
        templates: &HashMap<String, ReportTemplateModel>,
        results: &[SummaryResult],
    ) {
        let active = build_template_iterator(options, templates);

        if active.is_empty() {
            return;
        }

        let passed = results.iter().filter(|f| matches!(f, SummaryResult::Passed)).count();
        let failed = results.iter().filter(|f| matches!(f, SummaryResult::Failed)).count();
        let flakey = 0;

        for template in active{
            let summary = template.summary_template.clone().unwrap_or_default();
            let obj = liquid::object!({
                "passed": passed,
                "failed": failed,
                "flakey": flakey,
            });
            print_match(template, &summary, &obj);
        }

    }

    fn title(
        &self,
        options: &OptionsModel,
        templates: &HashMap<String, ReportTemplateModel>,
        test_count: usize,
    ) {
        let active = build_template_iterator(options, templates);

        if active.is_empty() {
            return;
        }

        for template in active{
            let title = template.title_template.clone().unwrap_or_default();
            let obj = liquid::object!({
                "test_count": test_count
            });
            print_match(template, &title, &obj);
        }
    }
}

fn build_template_iterator<'a>(
    options: &OptionsModel,
    templates: &'a HashMap<String, ReportTemplateModel>
) -> Vec<&'a ReportTemplateModel> {
    let report_names = options.reports.as_deref().unwrap_or_default();
    templates
        .iter()  // yields (&String, &ReportTemplateModel)
        .filter(|(key, _)| report_names.contains(&key.as_str().to_string()))
        .map(|(_, v)| v)
        .collect()
}

fn print(
    descriptor: &DescriptorModel,
    test_result: Option<&TestResult>,
    template: &ReportTemplateModel,
    assertions: &[Assertion],
    template_str: &str) {
    let globals = build_globals(descriptor, test_result, &assertions);
    print_match(template, template_str, &globals);
}

fn print_match(template: &ReportTemplateModel, template_str: &str, obj: &liquid::Object){
    match PARSER.parse(&template_str) {
        Ok(tmpl) => match tmpl.render(&obj) {
            Ok(output) => print!("{output}"),
            Err(e) => print_error(template, &format!("liquid parse error: {e}")),
        },
        Err(e) => print_error(template, &format!("liquid parse error: {e}")),
    };
}

fn print_error(template: &ReportTemplateModel, msg: &str){
    let obj = liquid::object!({"liquid_error_message" : msg});
    match PARSER.parse(&template.error_template.clone().unwrap_or_default()){
        Ok(tmpl) => match tmpl.render(&obj) {
            Ok(output) => print!("{output}"),
            Err(e) => eprintln!("{e}"),
        },
        Err(e) => eprintln!("{e}"),
    };
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
            "duration_ms":    result.duration.as_secs_f64() * 1000.0,
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
