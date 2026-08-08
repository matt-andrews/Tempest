use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidationDiagnostic {
    pub severity: ValidationSeverity,
    pub code: &'static str,
    pub path: Option<PathBuf>,
    pub context: Option<String>,
    pub message: String,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValidationSeverity {
    Error,
    Warning,
}

impl ValidationDiagnostic {
    pub fn error(
        code: &'static str,
        path: Option<PathBuf>,
        context: Option<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity: ValidationSeverity::Error,
            code,
            path,
            context,
            message: message.into(),
        }
    }
}
