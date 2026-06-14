use std::io;

use crossterm::event::{KeyEvent, MouseEvent};
use ratatui::layout::Rect;
use ratatui::Frame;

use super::action::Action;

pub type ActionResult = io::Result<Option<Action>>;

pub trait Component {
    fn handle_key_event(&mut self, _key: KeyEvent) -> Option<Action> {
        None
    }

    fn handle_mouse_event(&mut self, _mouse: MouseEvent) -> Option<Action> {
        None
    }

    fn update(&mut self, _action: &Action) -> Option<Action> {
        None
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> io::Result<()>;
}
