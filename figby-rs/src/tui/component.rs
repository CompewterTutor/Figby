use std::io;

use crossterm::event::{KeyEvent, MouseEvent};
use ratatui::layout::Rect;
use ratatui::Frame;

use super::events::AppEvent;

pub trait Component {
    fn handle_key_event(&mut self, _key: KeyEvent) -> Option<AppEvent> {
        None
    }

    fn handle_mouse_event(&mut self, _mouse: MouseEvent) -> Option<AppEvent> {
        None
    }

    fn update(&mut self, _event: &AppEvent) -> Option<AppEvent> {
        None
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> io::Result<()>;
}
