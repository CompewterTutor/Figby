const THROBBER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

#[derive(Clone)]
pub struct ThrobberState {
    frames: &'static [&'static str],
    frame_index: usize,
    pub active: bool,
    pub message: Option<String>,
}

impl ThrobberState {
    pub fn new() -> Self {
        Self {
            frames: THROBBER_FRAMES,
            frame_index: 0,
            active: false,
            message: None,
        }
    }

    pub fn tick(&mut self) {
        if !self.active {
            return;
        }
        self.frame_index = (self.frame_index + 1) % self.frames.len();
    }

    pub fn start(&mut self, msg: &str) {
        self.active = true;
        self.message = Some(msg.to_string());
        self.frame_index = 0;
    }

    pub fn stop(&mut self) {
        self.active = false;
        self.message = None;
        self.frame_index = 0;
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn render_string(&self) -> String {
        if !self.active {
            return String::new();
        }
        let spinner = self.frames[self.frame_index];
        match &self.message {
            Some(msg) => format!("{spinner} {msg}"),
            None => spinner.to_string(),
        }
    }
}

impl Default for ThrobberState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_throbber_tick_cycle() {
        let mut t = ThrobberState::new();
        t.start("test");
        let len = t.frames.len();
        for _ in 0..len {
            t.tick();
        }
        assert_eq!(t.frame_index, 0);
    }

    #[test]
    fn test_throbber_start_stop() {
        let mut t = ThrobberState::new();
        assert!(!t.is_active());
        t.start("Working");
        assert!(t.is_active());
        t.stop();
        assert!(!t.is_active());
    }

    #[test]
    fn test_throbber_render_active() {
        let mut t = ThrobberState::new();
        t.start("Saving");
        let rendered = t.render_string();
        assert!(rendered.contains("Saving"));
        // spinner should be one of the frames
        assert!(THROBBER_FRAMES.iter().any(|f| rendered.starts_with(f)));
    }

    #[test]
    fn test_throbber_render_inactive() {
        let t = ThrobberState::new();
        assert_eq!(t.render_string(), "");
    }

    #[test]
    fn test_throbber_tick_changes_frame() {
        let mut t = ThrobberState::new();
        t.start("test");
        let first = t.frames[t.frame_index];
        t.tick();
        let second = t.frames[t.frame_index];
        assert_ne!(first, second);
    }

    #[test]
    fn test_throbber_tick_inactive_noop() {
        let mut t = ThrobberState::new();
        t.tick();
        assert_eq!(t.frame_index, 0);
    }

    #[test]
    fn test_throbber_multiple_start_stop() {
        let mut t = ThrobberState::new();
        t.start("First");
        assert!(t.is_active());
        t.stop();
        assert!(!t.is_active());
        t.start("Second");
        assert!(t.is_active());
        assert_eq!(t.message.as_deref(), Some("Second"));
    }
}
