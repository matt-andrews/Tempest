use crate::validation::validation_diagnostic::ValidationDiagnostic;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidationReport {
    pub diagnostics: Vec<ValidationDiagnostic>,
    pub checked_specs: usize,
}
