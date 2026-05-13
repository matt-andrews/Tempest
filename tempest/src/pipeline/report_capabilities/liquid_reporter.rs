use std::collections::HashMap;
use std::sync::LazyLock;
use liquid::model::Value;
use crate::models::descriptor_model::DescriptorModel;
use crate::models::options_model::OptionsModel;
use crate::models::report_template_model::ReportTemplateModel;
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
        test_result: Option<TestResult>,
        assertions: Vec<Assertion>,
        options: OptionsModel,
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
                    &test_result,
                    template,
                    &assertions,
                    template.section_template.clone().unwrap_or_default()
                );
            }else {
                print(
                    descriptor,
                    &test_result,
                    template,
                    &assertions,
                    template.test_template.clone().unwrap_or_default()
                );
            }
        }
    }

    fn summary(&self) {
        todo!()
    }

    fn title(
        &self,
        options: OptionsModel,
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
            print_match(template, title, &obj);
        }
    }
}

fn build_template_iterator(
    options: OptionsModel,
    templates: &HashMap<String, ReportTemplateModel>
) -> Vec<&ReportTemplateModel>{
    let report_names = options.reports.unwrap_or_default();
    templates
        .iter()
        .filter(|(key, _)| report_names.contains(key))
        .map(|(_, v)| v)
        .collect()
}

fn print(
    descriptor: &DescriptorModel,
    test_result: &Option<TestResult>,
    template: &ReportTemplateModel,
    assertions: &Vec<Assertion>,
    template_str: String) {
    let globals = build_globals(descriptor, test_result.as_ref(), &assertions);
    print_match(template, template_str, &globals);
}

fn print_match(template: &ReportTemplateModel, template_str: String, obj: &liquid::Object){
    match PARSER.parse(&template_str) {
        Ok(tmpl) => match tmpl.render(&obj) {
            Ok(output) => print!("{output}"),
            Err(e) => print_error(template, format!("liquid parse error: {e}")),
        },
        Err(e) => print_error(template, format!("liquid parse error: {e}")),
    };
}

fn print_error(template: &ReportTemplateModel, msg: String){
    let obj = liquid::object!({"liquid_error_message" : msg.clone()});
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
