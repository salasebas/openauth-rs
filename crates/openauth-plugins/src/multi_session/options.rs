#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MultiSessionConfig {
    pub maximum_sessions: usize,
}

impl Default for MultiSessionConfig {
    fn default() -> Self {
        Self {
            maximum_sessions: 5,
        }
    }
}
