use crate::tui::canvas::{CanvasBuffer, CanvasCell};

pub fn sample(buffer: &CanvasBuffer, x: i16, y: i16) -> Option<CanvasCell> {
    if x < 0 || y < 0 {
        return None;
    }
    buffer.get(x as usize, y as usize).copied()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Color;

    #[test]
    fn test_sample_returns_cell_data() {
        let mut buf = CanvasBuffer::new(10, 10);
        let cell = CanvasCell {
            ch: '@',
            fg: Some(Color::Indexed(1)),
            bg: None,
            height: None,
        };
        buf.set(3, 5, cell);
        let sampled = sample(&buf, 3, 5).unwrap();
        assert_eq!(sampled.ch, '@');
        assert_eq!(sampled.fg, Some(Color::Indexed(1)));
    }

    #[test]
    fn test_sample_empty_cell_defaults() {
        let buf = CanvasBuffer::new(10, 10);
        let sampled = sample(&buf, 0, 0).unwrap();
        assert_eq!(sampled.ch, ' ');
        assert_eq!(sampled.fg, None);
    }

    #[test]
    fn test_sample_out_of_bounds_returns_none() {
        let buf = CanvasBuffer::new(10, 10);
        assert!(sample(&buf, -1, 0).is_none());
        assert!(sample(&buf, 0, -1).is_none());
        assert!(sample(&buf, 10, 0).is_none());
        assert!(sample(&buf, 0, 10).is_none());
    }

    #[test]
    fn test_sample_no_foreground() {
        let mut buf = CanvasBuffer::new(10, 10);
        let cell = CanvasCell {
            ch: 'X',
            fg: None,
            bg: None,
            height: None,
        };
        buf.set(2, 2, cell);
        let sampled = sample(&buf, 2, 2).unwrap();
        assert_eq!(sampled.ch, 'X');
        assert_eq!(sampled.fg, None);
    }

    #[test]
    fn test_sample_then_brush_char_updates() {
        let mut buf = CanvasBuffer::new(10, 10);
        let cell = CanvasCell {
            ch: '@',
            fg: None,
            bg: None,
            height: None,
        };
        buf.set(4, 6, cell);
        let sampled = sample(&buf, 4, 6).unwrap();
        assert_eq!(sampled.ch, '@');
    }
}
