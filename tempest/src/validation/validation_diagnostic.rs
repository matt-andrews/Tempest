use std::fmt;
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidationDiagnostic {
    pub severity: ValidationSeverity,
    pub code: &'static str,
    pub path: Option<PathBuf>,
    pub context: Option<String>,
    pub message: String,
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

    pub fn to_display(&self) -> String {
        if let Some(context) = &self.context {
            format!(
                "{} {}[{}]: {} ({})",
                self.path.as_deref().unwrap().display(),
                self.severity,
                self.code,
                self.message,
                context
            )
        } else {
            format!(
                "{} {}[{}]: {}",
                self.path.as_deref().unwrap().display(),
                self.severity,
                self.code,
                self.message
            )
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValidationSeverity {
    Error,
    Warning,
}
impl fmt::Display for ValidationSeverity {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ValidationSeverity::Error => write!(f, "error"),
            ValidationSeverity::Warning => write!(f, "warning"),
        }
    }
}
