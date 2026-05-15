use std::sync::LazyLock;
use liquid_core::Value;
use crate::models::descriptor::Descriptor;
use crate::models::report_template::ReportTemplate;
use crate::models::test_result::{Assertion, TestResult};
use crate::pipeline::reporting::event::ReportEvent;

pub static PARSER: LazyLock<liquid::Parser> = LazyLock::new(|| {
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

pub struct LiquidRenderer;
impl LiquidRenderer{
    pub fn render(
        &self,
        template: &ReportTemplate,
        event: &ReportEvent<'_>,
    ) -> anyhow::Result<String>{
        let template_str = event.template(template).unwrap_or_default();
        let globals = self.build_globals(event);
        let parsed = PARSER.parse(template_str)?;
        Ok(parsed.render(&globals)?)
    }

    fn build_globals(
        &self,
        event: &ReportEvent<'_>,
    ) -> liquid::Object {
        match event {
            ReportEvent::Title { test_count } => {
                liquid::object!({
                "test_count": *test_count,
            })
            }

            ReportEvent::Summary {
                passed,
                failed,
                flaky,
            } => {
                liquid::object!({
                "passed": *passed,
                "failed": *failed,
                "flaky": *flaky,
            })
            }

            ReportEvent::Descriptor {
                descriptor,
                test_result,
                assertions,
                test_count,
            } => build_descriptor_globals(
                descriptor,
                *test_result,
                assertions,
                *test_count,
            ),

            ReportEvent::Error {
                msg
            } => {
                liquid::object!({
                    "liquid_error_message": msg
                })
            }
        }
    }
}
fn build_descriptor_globals(
    descriptor: &Descriptor,
    test_result: Option<&TestResult>,
    assertions: &[Assertion],
    test_count: usize,
) -> liquid::Object {
    let all_passed = assertions.iter().all(|a| a.passed);

    let assertion_values: Vec<Value> = assertions
        .iter()
        .map(|a| {
            Value::Object(liquid::object!({
                "expr": a.expr.clone(),
                "passed": a.passed,
                "error": a.error.clone(),
            }))
        })
        .collect();

    let mut globals = if let Some(result) = test_result {
        liquid::object!({
            "name": descriptor.name.clone().unwrap_or_default(),
            "description": descriptor.description.clone().unwrap_or_default(),
            "passed": all_passed,
            "status": result.status.code as i64,
            "status_message": result.status.message.clone(),
            "body": result.body.clone(),
            "duration_ms": result.duration.as_secs_f64() * 1000.0,
            "test_count": test_count,
        })
    } else {
        liquid::object!({
            "name": descriptor.name.clone().unwrap_or_default(),
            "description": descriptor.description.clone().unwrap_or_default(),
            "passed": all_passed,
        })
    };

    globals.insert("assertions".into(), Value::Array(assertion_values));
    globals
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::descriptor::Descriptor;
    use crate::models::report_template::ReportTemplate;
    use crate::models::test_spec::TestSpec;
    use crate::models::test_result::{Assertion, TempestStatusCode, TestResult};
    use std::time::Duration;

    fn template(test_template: &str) -> ReportTemplate {
        ReportTemplate {
            test_template: Some(test_template.to_string()),
            section_template: Some(test_template.to_string()),
            error_template: Some(test_template.to_string()),
            title_template: Some(test_template.to_string()),
            summary_template: Some(test_template.to_string()),
            file: None,
        }
    }

    fn descriptor(has_test: bool) -> Descriptor {
        Descriptor {
            name: Some("login".to_string()),
            description: Some("login description".to_string()),
            tags: None,
            test: has_test.then(TestSpec::default),
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

    fn test_result() -> TestResult {
        TestResult {
            status: TempestStatusCode {
                code: 404,
                message: "Not Found".to_string(),
            },
            headers: reqwest::header::HeaderMap::new(),
            body: "missing".to_string(),
            json: None,
            bytes: vec![],
            duration: Duration::from_millis(250),
        }
    }

    #[test]
    fn renders_title_globals() {
        let renderer = LiquidRenderer;
        let event = ReportEvent::Title { test_count: 12 };

        let output = renderer
            .render(&template("{{ test_count }}"), &event)
            .unwrap();

        assert_eq!(output, "12");
    }

    #[test]
    fn renders_summary_globals() {
        let renderer = LiquidRenderer;
        let event = ReportEvent::Summary {
            passed: 8,
            failed: 2,
            flaky: 1,
        };

        let output = renderer
            .render(&template("{{ passed }}|{{ failed }}|{{ flaky }}"), &event)
            .unwrap();

        assert_eq!(output, "8|2|1");
    }

    #[test]
    fn renders_descriptor_globals_for_test_result() {
        let renderer = LiquidRenderer;
        let descriptor = descriptor(true);
        let result = test_result();
        let assertions = vec![
            assertion("status == 404u", true),
            assertion("body.contains(\"ok\")", false),
        ];

        let event = ReportEvent::Descriptor {
            descriptor: &descriptor,
            test_result: Some(&result),
            assertions: &assertions,
            test_count: 5,
        };

        let output = renderer
            .render(
                &template(
                    "{{ name }}|{{ description }}|{{ passed }}|{{ status }}|{{ status_message }}|{{ body }}|{{ test_count }}|{% for a in assertions %}{{ a.expr }}={{ a.passed }}:{{ a.error }};{% endfor %}",
                ),
                &event,
            )
            .unwrap();

        assert_eq!(
            output,
            "login|login description|false|404|Not Found|missing|5|status == 404u=true:;body.contains(\"ok\")=false:assertion failed;"
        );
    }

    #[test]
    fn renders_descriptor_globals_for_section_without_http_fields() {
        let renderer = LiquidRenderer;
        let descriptor = descriptor(false);

        let event = ReportEvent::Descriptor {
            descriptor: &descriptor,
            test_result: None,
            assertions: &[],
            test_count: 0,
        };

        let output = renderer
            .render(
                &template("{{ name }}|{% if status %}has-status{% else %}no-status{% endif %}"),
                &event,
            )
            .unwrap();

        assert_eq!(output, "login|no-status");
    }

    #[test]
    fn renders_error_globals() {
        let renderer = LiquidRenderer;
        let event = ReportEvent::Error {
            msg: "template failed",
        };

        let output = renderer
            .render(&template("ERR: {{ liquid_error_message }}"), &event)
            .unwrap();

        assert_eq!(output, "ERR: template failed");
    }

    #[test]
    fn returns_error_for_invalid_liquid_template() {
        let renderer = LiquidRenderer;
        let event = ReportEvent::Title { test_count: 1 };

        let result = renderer.render(&template("{{ broken"), &event);

        assert!(result.is_err());
    }
}
