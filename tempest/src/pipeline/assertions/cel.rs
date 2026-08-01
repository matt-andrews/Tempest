use crate::models::evaluation_context::EvaluationContext;
use crate::models::test_result::{Assertion, TestResult};
use crate::pipeline::assertions::AssertionEvaluator;
use anyhow::anyhow;
use cel_interpreter::{Program, Value};
use crate::pipeline::cel;
use crate::pipeline::cel::context;

pub struct CelAssertionEvaluator {
    assertion: String,
}
impl AssertionEvaluator for CelAssertionEvaluator {
    fn evaluate(&self, data: &TestResult, context: &EvaluationContext) -> Assertion {
        let result = Self::evaluate_assertion(&self.assertion, data, context)
            .map_err(|e| e.to_string());
        let error = match &result {
            Ok(_) => String::new(),
            Err(e) => e.clone(),
        };
        Assertion {
            expr: self.assertion.clone(),
            error,
            passed: result.unwrap_or(false),
        }
    }
}
impl CelAssertionEvaluator {
    pub fn new(assertion: &str) -> Self {
        Self {
            assertion: assertion.to_string(),
        }
    }

    fn evaluate_assertion(
        expr: &str,
        response: &TestResult,
        context: &EvaluationContext,
    ) -> anyhow::Result<bool> {
        match cel::evaluate(expr, response, context)? {
            Value::Bool(b) => Ok(b),
            other => anyhow::bail!("Assertion did not return a bool, got: {:?}", other),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::test_result::{TempestStatusCode, TestResult};
    use std::time::Duration;

    fn result(status: u16, body: &str) -> TestResult {
        TestResult {
            status: TempestStatusCode {
                code: status,
                message: "OK".to_string(),
            },
            headers: reqwest::header::HeaderMap::new(),
            body: body.to_string(),
            bytes: vec![],
            duration: Duration::ZERO,
        }
    }

    fn with_header(mut r: TestResult, name: &str, value: &str) -> TestResult {
        use reqwest::header::{HeaderName, HeaderValue};
        r.headers.insert(
            HeaderName::from_bytes(name.as_bytes()).unwrap(),
            HeaderValue::from_str(value).unwrap(),
        );
        r
    }

    fn eval(expr: &str, r: &TestResult) -> Assertion {
        CelAssertionEvaluator::new(expr).evaluate(r, &EvaluationContext::default())
    }

    #[test]
    fn status_equal_passes_and_preserves_expr() {
        let a = eval("status == 200u", &result(200, ""));
        assert!(a.passed);
        assert!(a.error.is_empty());
        assert_eq!(a.expr, "status == 200u");
    }

    #[test]
    fn status_not_equal_returns_false_with_no_error() {
        let a = eval("status == 404u", &result(200, ""));
        assert!(!a.passed);
        assert!(a.error.is_empty());
    }

    #[test]
    fn body_string_equality() {
        assert!(eval(r#"body == "hello""#, &result(200, "hello")).passed);
        assert!(!eval(r#"body == "hello""#, &result(200, "world")).passed);
    }

    #[test]
    fn body_contains_method() {
        let a = eval(r#"body.contains("ell")"#, &result(200, "hello"));
        assert!(a.passed);
    }

    #[test]
    fn header_value_comparison() {
        let r = with_header(result(200, ""), "x-request-id", "abc-123");
        assert!(eval(r#"headers["x-request-id"] == "abc-123""#, &r).passed);
    }

    #[test]
    fn compound_and_expression() {
        let pass = eval(r#"status == 200u && body == "ok""#, &result(200, "ok"));
        assert!(pass.passed);

        let fail = eval(r#"status == 200u && body == "ok""#, &result(200, "bad"));
        assert!(!fail.passed);
        assert!(fail.error.is_empty()); // still a valid CEL expression, just false
    }

    #[test]
    fn deprecated_variables_are_identified_from_program_references() {
        let program = Program::compile("json.name == 'Alice' && bytes.size() > 0").unwrap();
        let references = program.references();

        assert!(references.has_variable("json"));
        assert!(references.has_variable("bytes"));
    }

    #[test]
    fn deprecated_names_in_strings_or_fields_are_not_variable_references() {
        let program =
            Program::compile(r#"payload.json == "json" && payload.bytes == "bytes""#).unwrap();
        let references = program.references();

        assert!(!references.has_variable("json"));
        assert!(!references.has_variable("bytes"));
    }

    #[test]
    fn non_bool_expression_populates_error_and_sets_passed_false() {
        let a = eval("status + 1u", &result(200, ""));
        assert!(!a.passed);
        assert!(!a.error.is_empty());
    }

    // --- fileBytes ---

    fn eval_with_ctx(expr: &str, r: &TestResult, ctx: EvaluationContext) -> Assertion {
        CelAssertionEvaluator::new(expr).evaluate(r, &ctx)
    }

    fn suite_ctx(dir: &tempfile::TempDir) -> EvaluationContext {
        EvaluationContext {
            suite_dir: dir.path().to_path_buf(),
            spec_file: None,
        }
    }

    fn write_file(dir: &tempfile::TempDir, name: &str, contents: &[u8]) {
        std::fs::write(dir.path().join(name), contents).unwrap();
    }

    #[test]
    fn file_bytes_size_matches_file_length() {
        let dir = tempfile::tempdir().unwrap();
        write_file(&dir, "data.bin", b"hello");

        let a = eval_with_ctx(
            r#"fileBytes("/data.bin").size() == 5"#,
            &result(200, ""),
            suite_ctx(&dir),
        );
        assert!(a.passed);
        assert!(a.error.is_empty());
    }

    #[test]
    fn file_bytes_content_equals_known_bytes() {
        let dir = tempfile::tempdir().unwrap();
        write_file(&dir, "data.bin", b"hi");

        let a = eval_with_ctx(
            r#"fileBytes("/data.bin") == b"hi""#,
            &result(200, ""),
            suite_ctx(&dir),
        );
        assert!(a.passed, "fileBytes should return the exact file contents");
    }

    #[test]
    fn file_bytes_root_relative_resolves_from_suite_dir_regardless_of_spec() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("specs");
        std::fs::create_dir(&sub).unwrap();
        write_file(&dir, "root.bin", b"root");

        let ctx = EvaluationContext {
            suite_dir: dir.path().to_path_buf(),
            spec_file: Some(sub.join("test.spec.yml")),
        };
        let a = eval_with_ctx(
            r#"fileBytes("/root.bin").size() == 4"#,
            &result(200, ""),
            ctx,
        );
        assert!(
            a.passed,
            "/root.bin should resolve from suite_dir even with a spec_file set"
        );
    }

    #[test]
    fn file_bytes_spec_relative_resolves_from_spec_dir() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("specs");
        std::fs::create_dir(&sub).unwrap();
        std::fs::write(sub.join("payload.bin"), b"payload").unwrap();

        let ctx = EvaluationContext {
            suite_dir: dir.path().to_path_buf(),
            spec_file: Some(sub.join("test.spec.yml")),
        };
        let a = eval_with_ctx(
            r#"fileBytes("payload.bin").size() == 7"#,
            &result(200, ""),
            ctx,
        );
        assert!(
            a.passed,
            "spec-relative path should resolve from the spec file's directory"
        );
    }

    #[test]
    fn file_bytes_missing_file_fails_with_error() {
        let dir = tempfile::tempdir().unwrap();

        let a = eval_with_ctx(
            r#"fileBytes("/missing.bin")"#,
            &result(200, ""),
            suite_ctx(&dir),
        );
        assert!(!a.passed);
        assert!(
            !a.error.is_empty(),
            "missing file should populate the error field"
        );
    }

    #[test]
    fn file_bytes_empty_path_fails_with_error() {
        let a = eval_with_ctx(
            r#"fileBytes("")"#,
            &result(200, ""),
            EvaluationContext::default(),
        );
        assert!(!a.passed);
        assert!(!a.error.is_empty());
    }

    #[test]
    fn file_bytes_path_traversal_fails_with_error() {
        let dir = tempfile::tempdir().unwrap();

        let a = eval_with_ctx(
            r#"fileBytes("/../etc/passwd")"#,
            &result(200, ""),
            suite_ctx(&dir),
        );
        assert!(!a.passed);
        assert!(
            !a.error.is_empty(),
            "traversal escaping suite_dir should be rejected"
        );
    }

    #[test]
    fn file_bytes_can_be_used_in_compound_assertion() {
        let dir = tempfile::tempdir().unwrap();
        write_file(&dir, "token.bin", b"secret");

        let a = eval_with_ctx(
            r#"status == 200u && fileBytes("/token.bin") == b"secret""#,
            &result(200, ""),
            suite_ctx(&dir),
        );
        assert!(a.passed);
    }
}
