use crate::models::descriptor_model::DescriptorModel;
use crate::models::options_model::OptionsModel;
use crate::models::report_template_model::ReportTemplateModel;
use crate::models::summary_result::SummaryResult;
use crate::models::test_result::{Assertion, TestResult};
use crate::pipeline::report_capabilities::ReportCapability;
use liquid::model::Value;
use std::collections::HashMap;
use std::sync::LazyLock;
use crate::pipeline::report_capabilities::output_capabilities::{get_output, OutputCapability, OutputCapabilityProvider};

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
        .filter(OnRedFilter)
        .filter(OnGreenFilter)
        .filter(OnYellowFilter)
        .filter(OnBrightRedFilter)
        .filter(OnBrightGreenFilter)
        .filter(OnBrightBlueFilter)
        .filter(OnBrightPurpleFilter)
        .filter(ColorStatusFilter)
        .filter(ColorDurationFilter)
        .filter(JsonFilter)
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
        let active = self.build_template_iterator(options, templates);

        if active.is_empty() {
            return;
        }

        for template in active {
            let output_provider = &get_output(template, options);
            if descriptor.test.is_none() {
                self.print(
                    descriptor,
                    test_result,
                    template,
                    assertions,
                    &template.section_template.clone().unwrap_or_default(),
                    output_provider,
                );
            } else {
                self.print(
                    descriptor,
                    test_result,
                    template,
                    assertions,
                    &template.test_template.clone().unwrap_or_default(),
                    output_provider,
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
        let active = self.build_template_iterator(options, templates);

        if active.is_empty() {
            return;
        }

        let passed = results
            .iter()
            .filter(|f| matches!(f, SummaryResult::Passed))
            .count();
        let failed = results
            .iter()
            .filter(|f| matches!(f, SummaryResult::Failed))
            .count();
        let flaky = 0;

        for template in active {
            let output_provider = &get_output(template, options);
            let summary = template.summary_template.clone().unwrap_or_default();
            let obj = liquid::object!({
                "passed": passed,
                "failed": failed,
                "flaky": flaky,
            });
            self.print_match(template, &summary, &obj, output_provider);
        }
    }

    fn title(
        &self,
        options: &OptionsModel,
        templates: &HashMap<String, ReportTemplateModel>,
        test_count: usize,
    ) {
        let active = self.build_template_iterator(options, templates);

        if active.is_empty() {
            return;
        }

        for template in active {
            let output_provider = &get_output(template, options);
            let title = template.title_template.clone().unwrap_or_default();
            let obj = liquid::object!({
                "test_count": test_count
            });
            self.print_match(template, &title, &obj, output_provider);
        }
    }
}


impl LiquidReporter {
    fn print(
        &self,
        descriptor: &DescriptorModel,
        test_result: Option<&TestResult>,
        template: &ReportTemplateModel,
        assertions: &[Assertion],
        template_str: &str,
        output_provider: &OutputCapabilityProvider,
    ) {
        let globals = build_globals(descriptor, test_result, assertions);
        self.print_match(template, template_str, &globals, output_provider);
    }

    fn print_match(
        &self,
        template: &ReportTemplateModel,
        template_str: &str,
        obj: &liquid::Object,
        output_provider: &OutputCapabilityProvider,
    ) {
        match PARSER.parse(template_str) {
            Ok(tmpl) => match tmpl.render(&obj) {
                Ok(output) => output_provider.print(&output),
                Err(e) => self.print_error(template, &format!("liquid parse error: {e}"), output_provider),
            },
            Err(e) => self.print_error(template, &format!("liquid parse error: {e}"), output_provider),
        };
    }

    fn print_error(
        &self,
        template: &ReportTemplateModel,
        msg: &str,
        output_provider: &OutputCapabilityProvider,
    ) {
        let obj = liquid::object!({"liquid_error_message" : msg});
        match PARSER.parse(&template.error_template.clone().unwrap_or_default()) {
            Ok(tmpl) => match tmpl.render(&obj) {
                Ok(output) => output_provider.print(&output),
                Err(e) => output_provider.println(e.to_string().as_str()),
            },
            Err(e) => output_provider.println(e.to_string().as_str()),
        };
    }

    fn build_template_iterator<'a>(
        &self,
        options: &OptionsModel,
        templates: &'a HashMap<String, ReportTemplateModel>,
    ) -> Vec<&'a ReportTemplateModel> {
        let report_names = options.reports.as_deref().unwrap_or_default();
        templates
            .iter()
            .filter(|(key, _)| report_names.contains(&key.as_str().to_string()))
            .map(|(_, v)| v)
            .collect()
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
        .map(|a| {
            Value::Object(liquid::object!({
                "expr":   a.expr.clone(),
                "passed": a.passed,
                "error":  a.error.clone(),
            }))
        })
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::descriptor_model::DescriptorModel;
    use crate::models::options_model::OptionsModel;
    use crate::models::report_template_model::ReportTemplateModel;
    use crate::models::test_model::TestModel;
    use crate::models::test_result::{Assertion, TempestStatusCode, TestResult};
    use std::time::Duration;

    fn descriptor(name: &str, has_test: bool) -> DescriptorModel {
        DescriptorModel {
            name: Some(name.to_string()),
            description: Some(format!("{name} description")),
            tags: None,
            test: if has_test {
                Some(TestModel::default())
            } else {
                None
            },
            describe: None,
            options: None,
        }
    }

    fn assertion(expr: &str, passed: bool) -> Assertion {
        Assertion {
            expr: expr.to_string(),
            passed,
            error: if passed {
                String::new()
            } else {
                "assertion failed".to_string()
            },
        }
    }

    fn test_result(code: u16, body: &str, duration_ms: u64) -> TestResult {
        TestResult {
            status: TempestStatusCode {
                code,
                message: "OK".to_string(),
            },
            headers: reqwest::header::HeaderMap::new(),
            body: body.to_string(),
            json: None,
            bytes: vec![],
            duration: Duration::from_millis(duration_ms),
        }
    }

    fn make_template(test_tmpl: Option<&str>, section_tmpl: Option<&str>) -> ReportTemplateModel {
        ReportTemplateModel {
            test_template: test_tmpl.map(str::to_string),
            section_template: section_tmpl.map(str::to_string),
            error_template: Some("ERR:{{ liquid_error_message }}".to_string()),
            title_template: None,
            summary_template: None,
            file: None,
        }
    }

    fn options_reports(reports: &[&str]) -> OptionsModel {
        OptionsModel {
            base_uri: None,
            debug: None,
            reports: if reports.is_empty() {
                None
            } else {
                Some(reports.iter().map(|s| s.to_string()).collect())
            },
        }
    }

    fn render(template_str: &str, globals: &liquid::Object) -> String {
        PARSER.parse(template_str).unwrap().render(globals).unwrap()
    }

    #[test]
    fn globals_exposes_name_and_description() {
        let d = descriptor("login", false);
        let globals = build_globals(&d, None, &[]);
        assert_eq!(
            render("{{ name }}|{{ description }}", &globals),
            "login|login description"
        );
    }

    #[test]
    fn globals_name_defaults_to_empty_string_when_none() {
        let d = DescriptorModel {
            name: None,
            description: None,
            tags: None,
            test: None,
            describe: None,
            options: None,
        };
        let globals = build_globals(&d, None, &[]);
        assert_eq!(render("{{ name }}", &globals), "");
    }

    #[test]
    fn globals_passed_true_when_all_assertions_pass() {
        let d = descriptor("t", false);
        let globals = build_globals(
            &d,
            None,
            &[assertion("a == 1", true), assertion("b == 2", true)],
        );
        assert_eq!(render("{{ passed }}", &globals), "true");
    }

    #[test]
    fn globals_passed_false_when_any_assertion_fails() {
        let d = descriptor("t", false);
        let globals = build_globals(
            &d,
            None,
            &[assertion("a == 1", true), assertion("b == 99", false)],
        );
        assert_eq!(render("{{ passed }}", &globals), "false");
    }

    #[test]
    fn globals_with_test_result_exposes_status_body_and_duration() {
        let d = descriptor("t", true);
        let r = test_result(404, "not found", 250);
        let globals = build_globals(&d, Some(&r), &[]);
        assert_eq!(render("{{ status }}|{{ body }}", &globals), "404|not found");
        let ms: f64 = render("{{ duration_ms }}", &globals).parse().unwrap();
        assert!((ms - 250.0).abs() < 1.0);
    }

    #[test]
    fn globals_without_test_result_omits_http_fields() {
        let d = descriptor("t", false);
        let globals = build_globals(&d, None, &[]);
        // liquid raises on unknown vars; use conditionals to verify keys are absent
        assert_eq!(
            render("{% if status %}yes{% else %}no{% endif %}", &globals),
            "no"
        );
        assert_eq!(
            render("{% if body %}yes{% else %}no{% endif %}", &globals),
            "no"
        );
        assert_eq!(
            render("{% if duration_ms %}yes{% else %}no{% endif %}", &globals),
            "no"
        );
    }

    #[test]
    fn globals_assertions_array_is_iterable_with_expr_passed_and_error() {
        let d = descriptor("t", true);
        let r = test_result(200, "", 0);
        let assertions = vec![
            assertion("status == 200", true),
            assertion("body != ''", false),
        ];
        let globals = build_globals(&d, Some(&r), &assertions);
        let out = render(
            "{% for a in assertions %}{{ a.expr }}={{ a.passed }};{{ a.error }}|{% endfor %}",
            &globals,
        );
        assert_eq!(
            out,
            "status == 200=true;|body != ''=false;assertion failed|"
        );
    }

    #[test]
    fn template_iterator_returns_only_reports_named_in_options() {
        let mut templates = HashMap::new();
        templates.insert("console".to_string(), make_template(Some("C"), None));
        templates.insert("json".to_string(), make_template(Some("J"), None));
        templates.insert("html".to_string(), make_template(Some("H"), None));

        let active = LiquidReporter.build_template_iterator(&options_reports(&["console", "html"]), &templates);

        assert_eq!(active.len(), 2);
        let test_tmpls: Vec<_> = active
            .iter()
            .map(|t| t.test_template.as_deref().unwrap_or(""))
            .collect();
        assert!(test_tmpls.contains(&"C"));
        assert!(test_tmpls.contains(&"H"));
        assert!(!test_tmpls.contains(&"J"));
    }

    #[test]
    fn template_iterator_empty_when_reports_not_configured() {
        let mut templates = HashMap::new();
        templates.insert("console".to_string(), make_template(Some("C"), None));

        assert!(LiquidReporter.build_template_iterator(&options_reports(&[]), &templates).is_empty());
    }

    #[test]
    fn template_iterator_empty_when_no_template_name_matches() {
        let mut templates = HashMap::new();
        templates.insert("json".to_string(), make_template(Some("J"), None));

        assert!(LiquidReporter.build_template_iterator(&options_reports(&["xml"]), &templates).is_empty());
    }
}
