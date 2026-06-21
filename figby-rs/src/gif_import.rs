use crate::CanvasCell;
use gif::DisposalMethod;
use ratatui::style::Color;
use std::fs::File;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct GifImportResult {
    pub frames: Vec<Vec<Vec<CanvasCell>>>,
    pub frame_delays: Vec<u16>,
    pub loop_count: u16,
    pub palette_colors: Vec<Color>,
}

#[derive(Debug)]
pub enum GifImportError {
    Io(std::io::Error),
    Decode(String),
    NoFrames,
    TooLarge {
        width: usize,
        height: usize,
        frames: usize,
    },
}

impl std::fmt::Display for GifImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GifImportError::Io(e) => write!(f, "IO error: {e}"),
            GifImportError::Decode(e) => write!(f, "GIF decode error: {e}"),
            GifImportError::NoFrames => write!(f, "GIF has no frames"),
            GifImportError::TooLarge {
                width,
                height,
                frames,
            } => write!(f, "GIF too large: {width}x{height} x {frames} frames"),
        }
    }
}

impl std::error::Error for GifImportError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            GifImportError::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for GifImportError {
    fn from(e: std::io::Error) -> Self {
        GifImportError::Io(e)
    }
}

impl From<gif::DecodingError> for GifImportError {
    fn from(e: gif::DecodingError) -> Self {
        GifImportError::Decode(e.to_string())
    }
}

const MAX_TOTAL_CELLS: usize = 1_000_000;

pub fn import_gif(path: &Path) -> Result<GifImportResult, GifImportError> {
    let file = File::open(path)?;
    let mut decoder = gif::Decoder::new(file)?;

    let width = decoder.width() as usize;
    let height = decoder.height() as usize;

    if width == 0 || height == 0 {
        return Err(GifImportError::Decode("GIF has zero dimensions".into()));
    }

    // Guard: check per-frame cell count before reading any frames.
    if width.saturating_mul(height) > MAX_TOTAL_CELLS {
        return Err(GifImportError::TooLarge {
            width,
            height,
            frames: 1,
        });
    }

    let global_palette: Vec<[u8; 3]> = decoder
        .global_palette()
        .map(|pal| pal.chunks(3).map(|c| [c[0], c[1], c[2]]).collect())
        .unwrap_or_default();

    let bg_color_index = decoder.bg_color().unwrap_or(0);

    // Collect all frames, bailing as soon as cumulative cell count exceeds cap.
    let mut raw_frames: Vec<gif::Frame<'static>> = Vec::new();
    let mut frame_count: usize = 0;
    while let Some(frame) = decoder.read_next_frame()? {
        raw_frames.push(frame.clone());
        frame_count += 1;
        if width.saturating_mul(height).saturating_mul(frame_count) > MAX_TOTAL_CELLS {
            return Err(GifImportError::TooLarge {
                width,
                height,
                frames: frame_count,
            });
        }
    }

    if raw_frames.is_empty() {
        return Err(GifImportError::NoFrames);
    }

    let loop_count = match decoder.repeat() {
        gif::Repeat::Infinite => 0,
        gif::Repeat::Finite(n) => n,
    };

    let palette_colors: Vec<Color> = global_palette
        .iter()
        .map(|[r, g, b]| Color::Rgb(*r, *g, *b))
        .collect();

    // Background color for Dispose::Background
    let bg_color = global_palette
        .get(bg_color_index)
        .map(|[r, g, b]| Color::Rgb(*r, *g, *b));

    // Composite frames with proper disposal
    let empty_cell = CanvasCell {
        ch: ' ',
        fg: None,
        bg: bg_color,
        height: None,
    };
    let transparent = CanvasCell {
        ch: ' ',
        fg: None,
        bg: None,
        height: None,
    };
    let mut canvas = vec![vec![transparent; width]; height];
    let mut saved_canvas: Vec<Vec<CanvasCell>> = canvas.clone();
    let mut composited_frames: Vec<Vec<Vec<CanvasCell>>> = Vec::with_capacity(raw_frames.len());
    let mut delays: Vec<u16> = Vec::with_capacity(raw_frames.len());

    // Track properties of the previous frame for disposal
    let mut prev_dispose = DisposalMethod::Any;
    let mut prev_width: usize = 0;
    let mut prev_height: usize = 0;
    let mut prev_left: usize = 0;
    let mut prev_top: usize = 0;

    for frame in &raw_frames {
        delays.push(frame.delay);

        // Apply previous frame's dispose method before rendering this frame
        match prev_dispose {
            DisposalMethod::Background => {
                for row in canvas.iter_mut().skip(prev_top).take(prev_height) {
                    for cell in row.iter_mut().skip(prev_left).take(prev_width) {
                        *cell = empty_cell;
                    }
                }
            }
            DisposalMethod::Previous => {
                canvas.clone_from(&saved_canvas);
            }
            _ => {}
        }

        // Save state for future Dispose::Previous
        if frame.dispose == DisposalMethod::Previous {
            saved_canvas = canvas.clone();
        }

        // Track current frame's properties for next iteration's disposal
        prev_dispose = frame.dispose;
        prev_width = frame.width as usize;
        prev_height = frame.height as usize;
        prev_left = frame.left as usize;
        prev_top = frame.top as usize;

        // Determine palette for this frame
        let frame_palette: Vec<[u8; 3]> = frame
            .palette
            .as_ref()
            .map(|pal| pal.chunks(3).map(|c| [c[0], c[1], c[2]]).collect())
            .unwrap_or_else(|| global_palette.clone());

        let fw = frame.width as usize;
        let fh = frame.height as usize;
        let fl = frame.left as usize;
        let ft = frame.top as usize;

        // Render frame onto canvas
        if frame_palette.is_empty() {
            for y in 0..fh {
                let cy = ft + y;
                if cy >= height {
                    break;
                }
                for x in 0..fw {
                    let cx = fl + x;
                    if cx >= width {
                        break;
                    }
                    let idx = y * fw + x;
                    let Some(&pixel_value) = frame.buffer.get(idx) else {
                        continue;
                    };
                    let is_transparent =
                        frame.transparent.map(|t| pixel_value == t).unwrap_or(false);
                    if !is_transparent {
                        canvas[cy][cx] = CanvasCell {
                            ch: ' ',
                            fg: None,
                            bg: Some(Color::Rgb(pixel_value, pixel_value, pixel_value)),
                            height: None,
                        };
                    }
                }
            }
        } else {
            for y in 0..fh {
                let cy = ft + y;
                if cy >= height {
                    break;
                }
                for x in 0..fw {
                    let cx = fl + x;
                    if cx >= width {
                        break;
                    }
                    let idx = y * fw + x;
                    let Some(&raw_pixel) = frame.buffer.get(idx) else {
                        continue;
                    };
                    let pixel_idx = raw_pixel as usize;
                    let is_transparent = frame
                        .transparent
                        .map(|t| pixel_idx == t as usize)
                        .unwrap_or(false);
                    if !is_transparent && pixel_idx < frame_palette.len() {
                        let [r, g, b] = frame_palette[pixel_idx];
                        canvas[cy][cx] = CanvasCell {
                            ch: ' ',
                            fg: None,
                            bg: Some(Color::Rgb(r, g, b)),
                            height: None,
                        };
                    }
                }
            }
        }

        composited_frames.push(canvas.clone());
    }

    Ok(GifImportResult {
        frames: composited_frames,
        frame_delays: delays,
        loop_count,
        palette_colors,
    })
}
