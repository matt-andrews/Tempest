#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SummaryResult {
    Passed,
    Failed,
    Flaky,
    Skipped,
}
