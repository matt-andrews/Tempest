use crate::discovery::DiscoveryResult;
use crate::validation::validation_report::ValidationReport;

mod rules;
pub mod validation_diagnostic;
pub mod validation_report;

pub fn validate_project(project: &DiscoveryResult) -> ValidationReport {
    rules::run(project)
}
