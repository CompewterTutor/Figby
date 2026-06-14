use ratatui::style::Color;
use serde::Deserialize;

const DEFAULT_THEME_YAML: &str = include_str!("../../../assets/tui/themes/default.yaml");

pub fn color_from_hex(hex: &str) -> Color {
    let hex = hex.trim_start_matches('#');
    if hex.len() == 6 {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&hex[0..2], 16),
            u8::from_str_radix(&hex[2..4], 16),
            u8::from_str_radix(&hex[4..6], 16),
        ) {
            return Color::Rgb(r, g, b);
        }
    }
    Color::Reset
}

#[derive(Debug, Clone)]
pub struct Theme {
    pub toolbox: ToolboxTheme,
    pub canvas: CanvasTheme,
    pub palette: PaletteTheme,
    pub statusbar: StatusBarTheme,
    pub menu: MenuTheme,
    pub dialog: DialogTheme,
    pub general: GeneralTheme,
}

#[derive(Debug, Clone)]
pub struct ToolboxTheme {
    pub bg: Color,
    pub fg: Color,
    pub selected: Color,
}

#[derive(Debug, Clone)]
pub struct CanvasTheme {
    pub grid: Color,
    pub cursor: Color,
    pub selection: Color,
    pub edge: Color,
    pub text_block: Color,
}

#[derive(Debug, Clone)]
pub struct PaletteTheme {
    pub border: Color,
    pub active_target: Color,
    pub swatch_indicator: Color,
    pub cell_bg: Color,
}

#[derive(Debug, Clone)]
pub struct StatusBarTheme {
    pub mode_font: Color,
    pub mode_image: Color,
    pub mode_ascii: Color,
    pub separator: Color,
    pub label: Color,
}

#[derive(Debug, Clone)]
pub struct MenuTheme {
    pub bg: Color,
    pub fg: Color,
    pub highlight: Color,
    pub dim: Color,
    pub dropdown_bg: Color,
}

#[derive(Debug, Clone)]
pub struct DialogTheme {
    pub border_success: Color,
    pub border_path: Color,
    pub label: Color,
    pub meta: Color,
    pub error: Color,
    pub highlight: Color,
    pub selected_bg: Color,
}

#[derive(Debug, Clone)]
pub struct GeneralTheme {
    pub primary: Color,
    pub secondary: Color,
    pub success: Color,
    pub error: Color,
    pub warning: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            toolbox: ToolboxTheme {
                bg: Color::Reset,
                fg: Color::Reset,
                selected: Color::Cyan,
            },
            canvas: CanvasTheme {
                grid: Color::DarkGray,
                cursor: Color::Reset,
                selection: Color::Cyan,
                edge: Color::DarkGray,
                text_block: Color::Yellow,
            },
            palette: PaletteTheme {
                border: Color::DarkGray,
                active_target: Color::Yellow,
                swatch_indicator: Color::White,
                cell_bg: Color::DarkGray,
            },
            statusbar: StatusBarTheme {
                mode_font: Color::Blue,
                mode_image: Color::Green,
                mode_ascii: Color::Yellow,
                separator: Color::Reset,
                label: Color::Reset,
            },
            menu: MenuTheme {
                bg: Color::Reset,
                fg: Color::White,
                highlight: Color::Blue,
                dim: Color::DarkGray,
                dropdown_bg: Color::DarkGray,
            },
            dialog: DialogTheme {
                border_success: Color::Green,
                border_path: Color::Cyan,
                label: Color::Reset,
                meta: Color::DarkGray,
                error: Color::Red,
                highlight: Color::Yellow,
                selected_bg: Color::Reset,
            },
            general: GeneralTheme {
                primary: Color::Blue,
                secondary: Color::White,
                success: Color::Green,
                error: Color::Red,
                warning: Color::Yellow,
            },
        }
    }
}

// Intermediate YAML structs for serde deserialization
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct ThemeYaml {
    toolbox: Option<ToolboxYaml>,
    canvas: Option<CanvasYaml>,
    palette: Option<PaletteYaml>,
    statusbar: Option<StatusBarYaml>,
    menu: Option<MenuYaml>,
    dialog: Option<DialogYaml>,
    general: Option<GeneralYaml>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct ToolboxYaml {
    bg: Option<String>,
    fg: Option<String>,
    selected: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct CanvasYaml {
    grid: Option<String>,
    cursor: Option<String>,
    selection: Option<String>,
    edge: Option<String>,
    text_block: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct PaletteYaml {
    border: Option<String>,
    active_target: Option<String>,
    swatch_indicator: Option<String>,
    cell_bg: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct StatusBarYaml {
    mode_font: Option<String>,
    mode_image: Option<String>,
    mode_ascii: Option<String>,
    separator: Option<String>,
    label: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct MenuYaml {
    bg: Option<String>,
    fg: Option<String>,
    highlight: Option<String>,
    dim: Option<String>,
    dropdown_bg: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct DialogYaml {
    border_success: Option<String>,
    border_path: Option<String>,
    label: Option<String>,
    meta: Option<String>,
    error: Option<String>,
    highlight: Option<String>,
    selected_bg: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct GeneralYaml {
    primary: Option<String>,
    secondary: Option<String>,
    success: Option<String>,
    error: Option<String>,
    warning: Option<String>,
}

fn merge_color(yaml: Option<&String>, default: Color) -> Color {
    match yaml {
        Some(h) => color_from_hex(h),
        None => default,
    }
}

impl From<ThemeYaml> for Theme {
    fn from(y: ThemeYaml) -> Self {
        let base = Theme::default();
        Self {
            toolbox: ToolboxTheme {
                bg: merge_color(
                    y.toolbox.as_ref().and_then(|t| t.bg.as_ref()),
                    base.toolbox.bg,
                ),
                fg: merge_color(
                    y.toolbox.as_ref().and_then(|t| t.fg.as_ref()),
                    base.toolbox.fg,
                ),
                selected: merge_color(
                    y.toolbox.as_ref().and_then(|t| t.selected.as_ref()),
                    base.toolbox.selected,
                ),
            },
            canvas: CanvasTheme {
                grid: merge_color(
                    y.canvas.as_ref().and_then(|t| t.grid.as_ref()),
                    base.canvas.grid,
                ),
                cursor: merge_color(
                    y.canvas.as_ref().and_then(|t| t.cursor.as_ref()),
                    base.canvas.cursor,
                ),
                selection: merge_color(
                    y.canvas.as_ref().and_then(|t| t.selection.as_ref()),
                    base.canvas.selection,
                ),
                edge: merge_color(
                    y.canvas.as_ref().and_then(|t| t.edge.as_ref()),
                    base.canvas.edge,
                ),
                text_block: merge_color(
                    y.canvas.as_ref().and_then(|t| t.text_block.as_ref()),
                    base.canvas.text_block,
                ),
            },
            palette: PaletteTheme {
                border: merge_color(
                    y.palette.as_ref().and_then(|t| t.border.as_ref()),
                    base.palette.border,
                ),
                active_target: merge_color(
                    y.palette.as_ref().and_then(|t| t.active_target.as_ref()),
                    base.palette.active_target,
                ),
                swatch_indicator: merge_color(
                    y.palette.as_ref().and_then(|t| t.swatch_indicator.as_ref()),
                    base.palette.swatch_indicator,
                ),
                cell_bg: merge_color(
                    y.palette.as_ref().and_then(|t| t.cell_bg.as_ref()),
                    base.palette.cell_bg,
                ),
            },
            statusbar: StatusBarTheme {
                mode_font: merge_color(
                    y.statusbar.as_ref().and_then(|t| t.mode_font.as_ref()),
                    base.statusbar.mode_font,
                ),
                mode_image: merge_color(
                    y.statusbar.as_ref().and_then(|t| t.mode_image.as_ref()),
                    base.statusbar.mode_image,
                ),
                mode_ascii: merge_color(
                    y.statusbar.as_ref().and_then(|t| t.mode_ascii.as_ref()),
                    base.statusbar.mode_ascii,
                ),
                separator: merge_color(
                    y.statusbar.as_ref().and_then(|t| t.separator.as_ref()),
                    base.statusbar.separator,
                ),
                label: merge_color(
                    y.statusbar.as_ref().and_then(|t| t.label.as_ref()),
                    base.statusbar.label,
                ),
            },
            menu: MenuTheme {
                bg: merge_color(y.menu.as_ref().and_then(|t| t.bg.as_ref()), base.menu.bg),
                fg: merge_color(y.menu.as_ref().and_then(|t| t.fg.as_ref()), base.menu.fg),
                highlight: merge_color(
                    y.menu.as_ref().and_then(|t| t.highlight.as_ref()),
                    base.menu.highlight,
                ),
                dim: merge_color(y.menu.as_ref().and_then(|t| t.dim.as_ref()), base.menu.dim),
                dropdown_bg: merge_color(
                    y.menu.as_ref().and_then(|t| t.dropdown_bg.as_ref()),
                    base.menu.dropdown_bg,
                ),
            },
            dialog: DialogTheme {
                border_success: merge_color(
                    y.dialog.as_ref().and_then(|t| t.border_success.as_ref()),
                    base.dialog.border_success,
                ),
                border_path: merge_color(
                    y.dialog.as_ref().and_then(|t| t.border_path.as_ref()),
                    base.dialog.border_path,
                ),
                label: merge_color(
                    y.dialog.as_ref().and_then(|t| t.label.as_ref()),
                    base.dialog.label,
                ),
                meta: merge_color(
                    y.dialog.as_ref().and_then(|t| t.meta.as_ref()),
                    base.dialog.meta,
                ),
                error: merge_color(
                    y.dialog.as_ref().and_then(|t| t.error.as_ref()),
                    base.dialog.error,
                ),
                highlight: merge_color(
                    y.dialog.as_ref().and_then(|t| t.highlight.as_ref()),
                    base.dialog.highlight,
                ),
                selected_bg: merge_color(
                    y.dialog.as_ref().and_then(|t| t.selected_bg.as_ref()),
                    base.dialog.selected_bg,
                ),
            },
            general: GeneralTheme {
                primary: merge_color(
                    y.general.as_ref().and_then(|t| t.primary.as_ref()),
                    base.general.primary,
                ),
                secondary: merge_color(
                    y.general.as_ref().and_then(|t| t.secondary.as_ref()),
                    base.general.secondary,
                ),
                success: merge_color(
                    y.general.as_ref().and_then(|t| t.success.as_ref()),
                    base.general.success,
                ),
                error: merge_color(
                    y.general.as_ref().and_then(|t| t.error.as_ref()),
                    base.general.error,
                ),
                warning: merge_color(
                    y.general.as_ref().and_then(|t| t.warning.as_ref()),
                    base.general.warning,
                ),
            },
        }
    }
}

pub fn load_default() -> Theme {
    let yaml: ThemeYaml = serde_yaml::from_str(DEFAULT_THEME_YAML).unwrap_or_default();
    Theme::from(yaml)
}

pub fn load_custom(path: &str) -> Theme {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return load_default(),
    };
    let yaml: ThemeYaml = serde_yaml::from_str(&content).unwrap_or_default();
    Theme::from(yaml)
}

pub fn load_theme(theme_opt: &Option<String>) -> Theme {
    match theme_opt.as_deref() {
        None | Some("default") => load_default(),
        Some(path) => load_custom(path),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_load_default() {
        let theme = load_default();
        assert_ne!(theme.toolbox.bg, Color::Reset, "toolbox.bg should be set");
        assert_ne!(theme.canvas.grid, Color::Reset, "canvas.grid should be set");
        assert_ne!(
            theme.statusbar.mode_font,
            Color::Reset,
            "statusbar.mode_font should be set"
        );
        assert_ne!(
            theme.statusbar.mode_image,
            Color::Reset,
            "statusbar.mode_image should be set"
        );
        assert_ne!(
            theme.statusbar.mode_ascii,
            Color::Reset,
            "statusbar.mode_ascii should be set"
        );
        // Verify parsed to Rgb, not Reset
        if let Color::Rgb(r, g, b) = theme.toolbox.bg {
            assert_eq!(r, 0x1a);
            assert_eq!(g, 0x1b);
            assert_eq!(b, 0x26);
        } else {
            panic!("toolbox.bg should be Color::Rgb");
        }
        if let Color::Rgb(r, g, b) = theme.statusbar.mode_font {
            assert_eq!(r, 0x7a);
            assert_eq!(g, 0xa2);
            assert_eq!(b, 0xf7);
        } else {
            panic!("statusbar.mode_font should be Color::Rgb");
        }
        // Verify non-overridden defaults remain
        assert_eq!(theme.general.primary, Color::Rgb(0x7a, 0xa2, 0xf7));
        assert_eq!(theme.general.error, Color::Rgb(0xf7, 0x76, 0x8e));
    }

    #[test]
    fn test_theme_load_custom() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_theme_custom.yaml");
        let yaml = r##"
toolbox:
  bg: "#ff0000"
canvas:
  grid: "#00ff00"
statusbar:
  mode_font: "#0000ff"
"##;
        std::fs::write(&path, yaml).expect("write test theme");
        let theme = load_custom(path.to_str().unwrap());
        assert_eq!(theme.toolbox.bg, Color::Rgb(255, 0, 0));
        assert_eq!(theme.canvas.grid, Color::Rgb(0, 255, 0));
        assert_eq!(theme.statusbar.mode_font, Color::Rgb(0, 0, 255));
        // Non-overridden fields should still be at loaded values
        assert_ne!(theme.statusbar.mode_image, Color::Reset);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_theme_invalid_path() {
        let theme = load_custom("/nonexistent/path/theme.yaml");
        assert_ne!(
            theme.toolbox.bg,
            Color::Reset,
            "should fall back to default"
        );
    }

    #[test]
    fn test_theme_invalid_yaml() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_theme_bad.yaml");
        std::fs::write(&path, b"<<<garbage>>>").expect("write bad theme");
        let theme = load_custom(path.to_str().unwrap());
        assert_ne!(
            theme.toolbox.bg,
            Color::Reset,
            "should fall back to default"
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_load_theme_dispatcher() {
        let theme = load_theme(&None);
        assert_ne!(theme.toolbox.bg, Color::Reset);
        let theme = load_theme(&Some("default".to_string()));
        assert_ne!(theme.toolbox.bg, Color::Reset);
        let theme = load_theme(&Some("/nonexistent/theme.yaml".to_string()));
        assert_ne!(theme.toolbox.bg, Color::Reset, "fallback on bad path");
    }

    #[test]
    fn test_hex_parsing() {
        assert_eq!(color_from_hex("#ff0000"), Color::Rgb(255, 0, 0));
        assert_eq!(color_from_hex("#aabbcc"), Color::Rgb(170, 187, 204));
        assert_eq!(color_from_hex("#000000"), Color::Rgb(0, 0, 0));
        assert_eq!(color_from_hex(""), Color::Reset);
        assert_eq!(color_from_hex("nothex"), Color::Reset);
    }
}
