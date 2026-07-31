#[derive(Clone, PartialEq, Eq)]
pub enum SummaryResult {
    Passed,
    Failed,
    Flaky,
}
