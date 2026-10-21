#[derive(Debug, Copy, Clone)]
pub enum SlsqpError {
    IterationLimitExceeded = 9,
    MoreEqualityConstraints = 2,
    IterationLimitExceededLSQ = 3,
    IncompatibleConstraints = 4,
    SingularMatrixE = 5,
    SingularMatrixC = 6,
    RankDeficientHFTI = 7,
    PositiveDirectionalDerivative = 8,
}

impl SlsqpError {
    pub fn message(&self) -> &str {
        match self {
            Self::MoreEqualityConstraints => "More equality constraints than independent variables",
            Self::IterationLimitExceededLSQ => "More than 3*n iterations in LSQ subproblem",
            Self::IncompatibleConstraints => "Inequality constraints incompatible",
            Self::SingularMatrixE => "Singular matrix E in LSQ subproblem",
            Self::SingularMatrixC => "Singular matrix C in LSQ subproblem",
            Self::RankDeficientHFTI => "Rank-deficient equality constraint subproblem HFTI",
            Self::PositiveDirectionalDerivative => "Positive directional derivative for linesearch",
            Self::IterationLimitExceeded => "Iteration limit exceeded",
        }
    }
}
