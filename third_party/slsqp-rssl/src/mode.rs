#[cfg_attr(feature = "wasm", wasm_bindgen::prelude::wasm_bindgen)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum SlsqpMode {
    /// Initial mode
    Init = -2,
    /// Optimization terminated successfully.
    Success = 0,
    /// Function evaluation required (internal mode)
    LineSearch = 1,
    /// Gradient evaluation required (internal mode)
    EvalGrad = -1,
}

impl SlsqpMode {
    pub fn message(&self) -> &str {
        match self {
            SlsqpMode::Init => "Initializing solver.",
            SlsqpMode::Success => "Optimization terminated successfully.",
            SlsqpMode::LineSearch => "Function evaluation required (f & c)",
            SlsqpMode::EvalGrad => "Gradient evaluation required",
        }
    }
}

impl std::fmt::Display for SlsqpMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message())
    }
}
