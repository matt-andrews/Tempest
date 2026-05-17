use crate::models::descriptor::Descriptor;
use crate::models::report_template::ReportTemplate;
use crate::models::test_result::{Assertion, TestResult};

pub enum ReportEvent<'a> {
    Title {
        test_count: usize,
    },
    Descriptor {
        descriptor: &'a Descriptor,
        test_result: Option<&'a TestResult>,
        assertions: &'a [Assertion],
        test_count: usize,
    },
    Summary {
        passed: usize,
        failed: usize,
        flaky: usize,
    },
    Error {
        msg: &'a str,
    },
}

impl<'a> ReportEvent<'a> {
    pub fn template<'t>(&self, template: &'t ReportTemplate) -> Option<&'t str> {
        match self {
            ReportEvent::Title { .. } => template.title_template.as_deref(),
            ReportEvent::Descriptor { descriptor, .. } if descriptor.test.is_some() => {
                template.test_template.as_deref()
            }
            ReportEvent::Descriptor { .. } => template.section_template.as_deref(),
            ReportEvent::Summary { .. } => template.summary_template.as_deref(),
            ReportEvent::Error { .. } => template.error_template.as_deref(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::descriptor::Descriptor;
    use crate::models::report_template::ReportTemplate;
    use crate::models::test_spec::TestSpec;
    use std::path::PathBuf;

    fn template() -> ReportTemplate {
        ReportTemplate {
            title_template: Some("title".to_string()),
            test_template: Some("test".to_string()),
            section_template: Some("section".to_string()),
            summary_template: Some("summary".to_string()),
            error_template: Some("error".to_string()),
            file: None,
        }
    }

    fn descriptor(has_test: bool) -> Descriptor {
        Descriptor {
            name: Some("node".to_string()),
            description: None,
            tags: None,
            test: has_test.then(TestSpec::default),
            describe: None,
            options: None,
            file: None,
        }
    }

    #[test]
    fn title_event_selects_title_template() {
        let event = ReportEvent::Title { test_count: 3 };
        assert_eq!(event.template(&template()), Some("title"));
    }

    #[test]
    fn descriptor_event_with_test_selects_test_template() {
        let descriptor = descriptor(true);
        let event = ReportEvent::Descriptor {
            descriptor: &descriptor,
            test_result: None,
            assertions: &[],
            test_count: 1,
        };

        assert_eq!(event.template(&template()), Some("test"));
    }

    #[test]
    fn descriptor_event_without_test_selects_section_template() {
        let descriptor = descriptor(false);
        let event = ReportEvent::Descriptor {
            descriptor: &descriptor,
            test_result: None,
            assertions: &[],
            test_count: 0,
        };

        assert_eq!(event.template(&template()), Some("section"));
    }

    #[test]
    fn summary_event_selects_summary_template() {
        let event = ReportEvent::Summary {
            passed: 2,
            failed: 1,
            flaky: 0,
        };

        assert_eq!(event.template(&template()), Some("summary"));
    }

    #[test]
    fn error_event_selects_error_template() {
        let event = ReportEvent::Error { msg: "bad liquid" };
        assert_eq!(event.template(&template()), Some("error"));
    }
}
