#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RenderMode {
    Fast,
    #[default]
    Dirty,
}

impl RenderMode {
    pub fn toggle(self) -> Self {
        match self {
            RenderMode::Fast => RenderMode::Dirty,
            RenderMode::Dirty => RenderMode::Fast,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            RenderMode::Fast => "Fast",
            RenderMode::Dirty => "Dirty",
        }
    }

    pub fn poll_ms(self) -> u64 {
        match self {
            RenderMode::Fast | RenderMode::Dirty => 16,
        }
    }
}


