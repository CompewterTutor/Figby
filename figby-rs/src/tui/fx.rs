use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use std::time::Duration;
use tachyonfx::{fx, Effect, EffectTimer, Interpolation};

/// Stateful welcome-screen fade-in effect using tachyonfx.
pub struct WelcomeFx {
    effect: Effect,
}

impl WelcomeFx {
    pub fn new() -> Self {
        let timer = EffectTimer::from_ms(400, Interpolation::QuadOut);
        let effect = fx::fade_from_fg(Color::DarkGray, timer);
        Self { effect }
    }

    pub fn process(&mut self, dt: Duration, buf: &mut Buffer, area: Rect) {
        self.effect.process(dt, buf, area);
    }

    pub fn done(&self) -> bool {
        self.effect.done()
    }
}

impl Default for WelcomeFx {
    fn default() -> Self {
        Self::new()
    }
}
