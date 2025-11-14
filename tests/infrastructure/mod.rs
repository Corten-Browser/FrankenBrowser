pub mod test_recorder;
pub mod dashboard_generator;

pub use test_recorder::{TestRecorder, TestRun, BenchmarkResult, TestFailure, QualityMetric};
pub use dashboard_generator::DashboardGenerator;
