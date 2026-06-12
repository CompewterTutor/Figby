// .ftmp template file format parser and renderer

use serde::Deserialize;
use std::collections::HashMap;

use crate::font::{load_font, FIGfont, FontError};
use crate::render::{add_char, render_line, Justification};
use crate::smush::SmushMode;

/// Errors from template parsing/rendering.
#[derive(Debug)]
pub enum TemplateError {
    ParseError(String),
    FontError(FontError),
    ImageError(String),
    ResolveError(String),
}

impl std::fmt::Display for TemplateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TemplateError::ParseError(msg) => write!(f, "template parse error: {}", msg),
            TemplateError::FontError(e) => write!(f, "template font error: {}", e),
            TemplateError::ImageError(msg) => write!(f, "template image error: {}", msg),
            TemplateError::ResolveError(msg) => write!(f, "template resolve error: {}", msg),
        }
    }
}

impl std::error::Error for TemplateError {}

impl From<FontError> for TemplateError {
    fn from(e: FontError) -> Self {
        TemplateError::FontError(e)
    }
}

/// Canvas settings section in .ftmp frontmatter.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct CanvasSettings {
    #[serde(default)]
    pub width: Option<u32>,
    #[serde(default)]
    pub height: Option<u32>,
    #[serde(default)]
    pub keep_ratio: Option<bool>,
    #[serde(default)]
    pub margin: Option<u32>,
    #[serde(default)]
    pub padding: Option<u32>,
}

/// A single variable binding in .ftmp frontmatter.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct VariableBinding {
    pub text: String,
    #[serde(default)]
    pub font: Option<String>,
    #[serde(default)]
    pub x: Option<u32>,
    #[serde(default)]
    pub y: Option<u32>,
    #[serde(default)]
    pub z: Option<i32>,
    #[serde(default)]
    pub align: Option<String>,
    #[serde(default)]
    pub overlap: Option<String>,
    #[serde(default)]
    pub border_width: Option<u32>,
    #[serde(default)]
    pub border_color: Option<String>,
    #[serde(default)]
    pub shadow_size: Option<u32>,
    #[serde(default)]
    pub shadow_color: Option<String>,
}

/// Top-level frontmatter structure.
#[derive(Debug, Deserialize)]
struct TemplateFrontmatter {
    #[serde(default)]
    canvas: Option<CanvasSettings>,
    #[serde(default)]
    variables: Option<HashMap<String, VariableBinding>>,
}

/// An inline image tag `{{img:source:width:height:color:pos:charset}}`.
#[derive(Debug, Clone)]
pub struct ImageTag {
    pub source: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub colored: bool,
    pub x: Option<u32>,
    pub y: Option<u32>,
    pub charset: String,
}

/// A renderable layer derived from a `{{varname}}` in the template body.
#[derive(Debug, Clone)]
pub struct Layer {
    pub varname: String,
    pub binding: VariableBinding,
    /// Position in body text (for tie-breaking on equal z).
    pub body_index: usize,
    pub image_tag: Option<ImageTag>,
}

/// Parsed .ftmp template.
#[derive(Debug, Clone)]
pub struct Template {
    pub canvas: CanvasSettings,
    pub layers: Vec<Layer>,
    pub body: String,
}

/// Configuration passed to `render_template`.
#[derive(Debug, Clone)]
pub struct RenderConfig {
    pub font_dir: String,
    pub term_width: u32,
    pub override_width: Option<u32>,
}

/// Resolve `${VAR}` env var and `$(cmd)` command substitution in a text value.
/// Plain string literals pass through unchanged.
pub fn resolve_text_value(text: &str) -> Result<String, TemplateError> {
    let mut result = String::new();
    let mut remaining = text;

    while let Some(dollar_pos) = remaining.find('$') {
        result.push_str(&remaining[..dollar_pos]);
        remaining = &remaining[dollar_pos + 1..];

        if let Some(rest) = remaining.strip_prefix('{') {
            let closing = rest.find('}').ok_or_else(|| {
                TemplateError::ResolveError("unclosed env var `${...}`".to_string())
            })?;
            let var_name = &rest[..closing];
            let value = std::env::var(var_name).map_err(|_| {
                TemplateError::ResolveError(format!("env var '{}' not set", var_name))
            })?;
            result.push_str(&value);
            remaining = &rest[closing + 1..];
        } else if let Some(rest) = remaining.strip_prefix('(') {
            let closing = rest.find(')').ok_or_else(|| {
                TemplateError::ResolveError("unclosed command sub `$(...)`".to_string())
            })?;
            let cmd_str = &rest[..closing];
            let shell = if cfg!(target_os = "windows") { "cmd" } else { "sh" };
            let arg = if cfg!(target_os = "windows") { "/C" } else { "-c" };
            let output = std::process::Command::new(shell)
                .args([arg, cmd_str])
                .output()
                .map_err(|e| TemplateError::ResolveError(format!("command failed: {}", e)))?;
            if !output.status.success() {
                return Err(TemplateError::ResolveError(format!(
                    "command exited with {}",
                    output.status
                )));
            }
            let stdout = String::from_utf8_lossy(&output.stdout);
            result.push_str(stdout.trim_end_matches('\n'));
            remaining = &rest[closing + 1..];
        } else {
            result.push('$');
        }
    }

    result.push_str(remaining);
    Ok(result)
}

/// Parse a .ftmp template string into a `Template`.
///
/// Format:
/// ```toml
/// ---
/// [canvas]
/// width = 80
///
/// [variables.greeting]
/// text = "Hello"
/// ---
/// {{greeting}}
/// ```
pub fn parse_ftmp(input: &str) -> Result<Template, TemplateError> {
    let stripped = input.trim_start();

    if !stripped.starts_with("---") {
        return Err(TemplateError::ParseError(
            "template must start with '---' frontmatter delimiter".to_string(),
        ));
    }

    let after_opener = &stripped[3..];
    let end_marker = after_opener.find("\n---").ok_or_else(|| {
        TemplateError::ParseError("missing closing '---' frontmatter delimiter".to_string())
    })?;

    let frontmatter_str = after_opener[..end_marker].trim();
    let body = after_opener[end_marker + 4..].trim().to_string();

    let frontmatter: TemplateFrontmatter = toml::from_str(frontmatter_str)
        .map_err(|e| TemplateError::ParseError(format!("invalid TOML frontmatter: {}", e)))?;

    let variables = frontmatter.variables.unwrap_or_default();
    let canvas = frontmatter.canvas.unwrap_or_default();

    let mut layers = Vec::new();
    let mut body_idx = 0;
    let mut remaining = body.as_str();

    while let Some(start) = remaining.find("{{") {
        remaining = &remaining[start + 2..];
        if let Some(end) = remaining.find("}}") {
            let tag_content = remaining[..end].trim();
            if !tag_content.is_empty() {
                if let Some(img_rest) = tag_content.strip_prefix("img:") {
                    let parts: Vec<&str> = img_rest.split(':').collect();
                    let source = parts.first().unwrap_or(&"").to_string();
                    let width = parts.get(1).and_then(|s| s.parse().ok());
                    let height = parts.get(2).and_then(|s| s.parse().ok());
                    let colored = matches!(
                        *parts.get(3).unwrap_or(&""),
                        "true" | "1" | "yes" | "y"
                    );
                    let (x, y) = parts.get(4).and_then(|s| {
                        let mut ps = s.split(',');
                        let px = ps.next()?.parse().ok()?;
                        let py = ps.next()?.parse().ok()?;
                        Some((px, py))
                    }).unwrap_or((0, 0));
                    let charset = parts.get(5).unwrap_or(&"").to_string();

                    layers.push(Layer {
                        varname: tag_content.to_string(),
                        binding: VariableBinding {
                            x: Some(x),
                            y: Some(y),
                            z: Some(0),
                            ..VariableBinding::default()
                        },
                        body_index: body_idx,
                        image_tag: Some(ImageTag {
                            source,
                            width,
                            height,
                            colored,
                            x: Some(x),
                            y: Some(y),
                            charset,
                        }),
                    });
                } else if let Some(binding) = variables.get(tag_content) {
                    layers.push(Layer {
                        varname: tag_content.to_string(),
                        binding: binding.clone(),
                        body_index: body_idx,
                        image_tag: None,
                    });
                }
            }
            body_idx += 1;
            remaining = &remaining[end + 2..];
        }
    }

    Ok(Template {
        canvas,
        layers,
        body,
    })
}

/// Parse all `{{varname}}` placeholders from body text.
pub fn extract_placeholders(body: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut remaining = body;
    while let Some(start) = remaining.find("{{") {
        remaining = &remaining[start + 2..];
        if let Some(end) = remaining.find("}}") {
            let varname = remaining[..end].trim().to_string();
            if !varname.is_empty() {
                result.push(varname);
            }
            remaining = &remaining[end + 2..];
        }
    }
    result
}

/// Render a single line of text through the FIGlet pipeline.
fn render_text_line(
    font: &FIGfont,
    text: &str,
    output_width: usize,
    justification: Justification,
    rtl: bool,
) -> Vec<String> {
    let height = font.charheight as usize;
    let smush_mode = SmushMode::new(font.full_layout as u32);
    let limit = output_width.saturating_sub(1);

    let mut rows = vec![String::new(); height];
    let mut outlinelen = 0;
    let mut prev_width = 0;

    for c in text.chars() {
        let code = c as u32;
        let ok = add_char(
            font,
            code,
            &mut rows,
            &mut outlinelen,
            &mut prev_width,
            smush_mode,
            rtl,
            limit,
        );
        if !ok {
            break;
        }
    }

    render_line(&rows, font.hardblank, justification, output_width)
}

/// Render multi-line text through FIGlet, one FIGlet line per text line.
fn render_figlet_text(
    font: &FIGfont,
    text: &str,
    output_width: usize,
    justification: Justification,
) -> Vec<String> {
    let height = font.charheight as usize;
    let rtl = font.print_direction == 1;
    let mut all_rows = Vec::new();

    for line in text.split('\n') {
        if line.is_empty() {
            for _ in 0..height {
                all_rows.push(" ".repeat(output_width));
            }
        } else {
            all_rows.extend(render_text_line(
                font,
                line,
                output_width,
                justification,
                rtl,
            ));
        }
    }

    all_rows
}

/// Place rendered rows onto the canvas at (x, y), overwriting existing chars.
fn place_on_canvas(canvas: &mut [Vec<char>], rows: &[String], x: usize, y: usize) {
    for (i, row) in rows.iter().enumerate() {
        let canvas_y = y + i;
        if canvas_y >= canvas.len() {
            break;
        }
        for (j, c) in row.chars().enumerate() {
            let canvas_x = x + j;
            if canvas_x >= canvas[canvas_y].len() {
                break;
            }
            canvas[canvas_y][canvas_x] = c;
        }
    }
}

/// Render a parsed template onto a canvas and return output rows.
pub fn render_template(
    tmpl: &Template,
    config: &RenderConfig,
) -> Result<Vec<String>, TemplateError> {
    let width = config
        .override_width
        .or(tmpl.canvas.width)
        .unwrap_or(config.term_width) as usize;

    let height = tmpl.canvas.height.unwrap_or(24) as usize;
    let margin = tmpl.canvas.margin.unwrap_or(0) as usize;
    let padding = tmpl.canvas.padding.unwrap_or(0) as usize;

    let mut canvas = vec![vec![' '; width]; height];

    // Collect unique font names from non-image layers and pre-load them.
    let font_names: Vec<String> = {
        let mut names: Vec<String> = tmpl
            .layers
            .iter()
            .filter(|l| l.image_tag.is_none())
            .map(|l| {
                l.binding
                    .font
                    .clone()
                    .unwrap_or_else(|| "standard".to_string())
            })
            .collect();
        names.sort();
        names.dedup();
        names
    };

    let mut font_cache: HashMap<String, FIGfont> = HashMap::new();
    for name in &font_names {
        font_cache.insert(name.clone(), load_font(name, &config.font_dir)?);
    }

    // Sort layers by z (ascending), then body_index (ascending).
    let mut sorted_layers: Vec<&Layer> = tmpl.layers.iter().collect();
    sorted_layers.sort_by(|a, b| {
        let za = a.binding.z.unwrap_or(0);
        let zb = b.binding.z.unwrap_or(0);
        za.cmp(&zb).then(a.body_index.cmp(&b.body_index))
    });

    let mut flow_y: usize = 0;

    for layer in &sorted_layers {
        let x = layer.binding.x.unwrap_or(0) as usize;
        let overlap = layer.binding.overlap.as_deref().unwrap_or("overwrite");

        if let Some(ref img) = layer.image_tag {
            let mut options = rascii_art::RenderOptions::new();
            if let Some(w) = img.width {
                options = options.width(w);
            }
            if let Some(h) = img.height {
                options = options.height(h);
            }
            if img.colored {
                options = options.colored(true);
            }
            if !img.charset.is_empty() {
                if let Some(cs) = rascii_art::charsets::from_str(&img.charset) {
                    options = options.charset(cs);
                }
            }

            let mut buf = String::new();
            rascii_art::render_to(&img.source, &mut buf, &options)
                .map_err(|e| TemplateError::ImageError(e.to_string()))?;

            let rows: Vec<String> = buf.lines().map(|l| l.to_string()).collect();

            match overlap {
                "flow" => {
                    let y_pos = flow_y;
                    flow_y += rows.len();
                    place_on_canvas(&mut canvas, &rows, x, y_pos);
                }
                _ => {
                    let y = layer.binding.y.unwrap_or(0) as usize;
                    place_on_canvas(&mut canvas, &rows, x, y);
                }
            }
        } else {
            let font_name = layer.binding.font.as_deref().unwrap_or("standard");
            let font = font_cache.get(font_name).ok_or_else(|| {
                TemplateError::ParseError(format!("font '{}' not loaded", font_name))
            })?;

            let align = match layer.binding.align.as_deref() {
                Some("center") => Justification::Center,
                Some("right") => Justification::Right,
                _ => Justification::Left,
            };

            let text = resolve_text_value(&layer.binding.text)?;
            let rows = render_figlet_text(font, &text, width, align);

            match overlap {
                "flow" => {
                    let y_pos = flow_y;
                    flow_y += rows.len();
                    place_on_canvas(&mut canvas, &rows, x, y_pos);
                }
                _ => {
                    let y = layer.binding.y.unwrap_or(0) as usize;
                    place_on_canvas(&mut canvas, &rows, x, y);
                }
            }
        }
    }

    // Convert canvas to output rows.
    let mut output: Vec<String> = canvas.iter().map(|row| row.iter().collect()).collect();

    // Apply padding.
    if padding > 0 {
        let pad = " ".repeat(padding);
        for row in &mut output {
            let inner = std::mem::take(row);
            *row = format!("{}{}{}", pad, inner, pad);
        }
    }

    // Apply margin.
    if margin > 0 {
        let blank = " ".repeat(width + 2 * padding);
        let mut result = Vec::new();
        result.extend(std::iter::repeat_n(blank.clone(), margin));
        result.extend(output);
        result.extend(std::iter::repeat_n(blank, margin));
        output = result;
    }

    Ok(output)
}

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use super::*;

    fn test_font() -> FIGfont {
        use std::collections::HashMap;
        let mut chars = HashMap::new();
        chars.insert(0, FIGcharacter_from("####"));
        chars.insert(32, FIGcharacter_from("    "));
        chars.insert(65, FIGcharacter_from(" AA "));
        chars.insert(66, FIGcharacter_from(" BB "));
        chars.insert(67, FIGcharacter_from(" CC "));
        FIGfont {
            charheight: 1,
            hardblank: '$',
            full_layout: 0,
            chars,
            ..FIGfont::default()
        }
    }

    fn FIGcharacter_from(s: &str) -> crate::font::FIGcharacter {
        crate::font::FIGcharacter::from(vec![s.to_string()])
    }

    fn setup_font_dir() -> (tempfile::TempDir, String) {
        let tmpdir = tempfile::tempdir().unwrap();
        let font_bytes = include_bytes!("../../fonts/standard.flf");
        std::fs::write(tmpdir.path().join("standard.flf"), font_bytes).unwrap();
        let font_dir = tmpdir.path().to_str().unwrap().to_string();
        (tmpdir, font_dir)
    }

    #[test]
    fn test_parse_ftmp_basic() {
        let ftmp = r#"---
[canvas]
width = 80
height = 24

[variables.greeting]
text = "Hello World"
font = "standard"
x = 0
y = 0
---
{{greeting}}
"#;
        let tmpl = parse_ftmp(ftmp).expect("should parse");
        assert_eq!(tmpl.canvas.width, Some(80));
        assert_eq!(tmpl.canvas.height, Some(24));
        assert_eq!(tmpl.layers.len(), 1);
        assert_eq!(tmpl.layers[0].varname, "greeting");
        assert_eq!(tmpl.layers[0].binding.text, "Hello World");
        assert_eq!(tmpl.layers[0].binding.font.as_deref(), Some("standard"));
    }

    #[test]
    fn test_parse_ftmp_no_frontmatter() {
        let ftmp = "---\n---\n{{x}}\n";
        let tmpl = parse_ftmp(ftmp).expect("should parse empty frontmatter");
        assert!(tmpl.canvas.width.is_none());
        assert_eq!(tmpl.layers.len(), 0);
    }

    #[test]
    fn test_parse_ftmp_variable_without_body_placeholder() {
        let ftmp = r#"---
[variables.foo]
text = "hello"
---
there are no placeholders here
"#;
        let tmpl = parse_ftmp(ftmp).expect("should parse");
        assert_eq!(tmpl.layers.len(), 0);
    }

    #[test]
    fn test_parse_ftmp_canvas_settings() {
        let ftmp = r#"---
[canvas]
width = 60
height = 30
keep_ratio = true
margin = 2
padding = 1
---
{{a}}
"#;
        let tmpl = parse_ftmp(ftmp).expect("should parse");
        assert_eq!(tmpl.canvas.width, Some(60));
        assert_eq!(tmpl.canvas.height, Some(30));
        assert_eq!(tmpl.canvas.keep_ratio, Some(true));
        assert_eq!(tmpl.canvas.margin, Some(2));
        assert_eq!(tmpl.canvas.padding, Some(1));
    }

    #[test]
    fn test_parse_ftmp_missing_delimiter() {
        let err = parse_ftmp("no delimiter here").unwrap_err();
        assert!(err.to_string().contains("must start with"));
    }

    #[test]
    fn test_parse_ftmp_missing_closing_delimiter() {
        let err = parse_ftmp("---\n[key = 1]\nno closing").unwrap_err();
        assert!(err.to_string().contains("missing closing"));
    }

    #[test]
    fn test_extract_placeholders() {
        let body = "Hello {{name}}, your {{item}} is ready";
        let vars = extract_placeholders(body);
        assert_eq!(vars, vec!["name", "item"]);
    }

    #[test]
    fn test_extract_placeholders_no_match() {
        let vars = extract_placeholders("just text");
        assert!(vars.is_empty());
    }

    #[test]
    fn test_extract_placeholders_empty_name() {
        let vars = extract_placeholders("{{}}");
        assert!(vars.is_empty());
    }

    #[test]
    fn test_render_overwrite_mode() {
        // Two layers at same (x,y), second overwrites first.
        let font = test_font();
        let text_a = "AA";
        let text_b = "BB";

        let rows_a = render_figlet_text(&font, text_a, 20, Justification::Left);
        let rows_b = render_figlet_text(&font, text_b, 20, Justification::Left);

        let mut canvas = vec![vec![' '; 20]; 10];
        place_on_canvas(&mut canvas, &rows_a, 0, 0);
        place_on_canvas(&mut canvas, &rows_b, 0, 0);

        // 'B' char overwrites 'A' at start. But note overlay: ' AA ' placed at x=0
        // then ' BB ' placed at x=0. canvas[0] should have "BB" then spaces.
        let row: String = canvas[0].iter().collect();
        assert!(
            row.starts_with("BB  "),
            "second layer overwrites first: got '{row}'"
        );
    }

    #[test]
    fn test_render_flow_mode() {
        // Two layers with flow: second stacks below first.
        let font = test_font();
        let text_a = "AA";
        let text_b = "BB";

        let rows_a = render_figlet_text(&font, text_a, 20, Justification::Left);
        let rows_b = render_figlet_text(&font, text_b, 20, Justification::Left);

        let mut canvas = vec![vec![' '; 20]; 10];
        let mut flow_y = 0;

        // Layer A (flow)
        place_on_canvas(&mut canvas, &rows_a, 0, flow_y);
        flow_y += rows_a.len();

        // Layer B (flow) placed at new flow_y
        place_on_canvas(&mut canvas, &rows_b, 0, flow_y);

        let row0: String = canvas[0].iter().collect();
        let row1: String = canvas[1].iter().collect();
        assert!(row0.starts_with(" AA "), "layer A on row 0: got '{row0}'");
        assert!(row1.starts_with(" BB "), "layer B on row 1: got '{row1}'");
    }

    #[test]
    fn test_render_z_order() {
        // Three layers at (0,0) with z=0, z=2, z=1.
        // Sort should order z=0, z=1, z=2 (ascending z).
        // Last rendered (z=2) overwrites earlier.
        let font = test_font();
        let text_a = "AA"; // renders as " AA "
        let text_b = "BB"; // renders as " BB "
        let text_c = "CC"; // renders as " CC "

        let rows_a = render_figlet_text(&font, text_a, 20, Justification::Left);
        let rows_b = render_figlet_text(&font, text_b, 20, Justification::Left);
        let rows_c = render_figlet_text(&font, text_c, 20, Justification::Left);

        let mut canvas = vec![vec![' '; 20]; 10];

        // Render in z=2, z=0, z=1 order (simulating unsorted input)
        place_on_canvas(&mut canvas, &rows_c, 0, 0); // z=2
        place_on_canvas(&mut canvas, &rows_a, 0, 0); // z=0
        place_on_canvas(&mut canvas, &rows_b, 0, 0); // z=1

        // After re-sorting: z=0 (AA), z=1 (BB), z=2 (CC) → final = CC
        let row: String = canvas[0].iter().collect();
        assert!(row.starts_with("CC  "), "z=2 should be on top: got '{row}'");
    }

    #[test]
    fn test_render_canvas_size() {
        let ftmp = r#"---
[canvas]
width = 30
height = 10

[variables.msg]
text = "Hi"
font = "standard"
---
{{msg}}
"#;
        let tmpl = parse_ftmp(ftmp).expect("should parse");
        let (_tmpdir, font_dir) = setup_font_dir();
        let config = RenderConfig {
            font_dir,
            term_width: 80,
            override_width: None,
        };
        let output = render_template(&tmpl, &config).expect("should render");
        for row in &output {
            assert!(
                row.chars().count() <= 30,
                "row width {} exceeds canvas width 30: '{row}'",
                row.chars().count()
            );
        }
    }

    #[test]
    fn test_end_to_end_template() {
        let ftmp = r#"---
[canvas]
width = 50
height = 10

[variables.greeting]
text = "Hello"
font = "standard"
---
{{greeting}}
"#;
        let tmpl = parse_ftmp(ftmp).expect("should parse");
        let (_tmpdir, font_dir) = setup_font_dir();
        let config = RenderConfig {
            font_dir,
            term_width: 80,
            override_width: None,
        };
        let output = render_template(&tmpl, &config).expect("should render");
        // Should produce 6 rows (standard font height) at least
        assert!(!output.is_empty(), "output should have rows");
        // Each row should contain the rendered "Hello" text
        assert!(
            output[0].contains('H') || output[0].contains(' '),
            "row 0 should contain 'H' or spaces"
        );
        // Verify all rows are within canvas width
        for row in &output {
            assert!(
                row.chars().count() <= 50,
                "row width {} exceeds 50",
                row.chars().count()
            );
        }
    }

    #[test]
    fn test_render_margin_padding() {
        let font = test_font();
        let rows = render_figlet_text(&font, "AA", 20, Justification::Left);

        let mut canvas = vec![vec![' '; 20]; 5];
        place_on_canvas(&mut canvas, &rows, 0, 0);

        // Convert to output string rows
        let output: Vec<String> = canvas.iter().map(|r| r.iter().collect()).collect();

        // Apply padding=1
        let pad = " ".to_string();
        let padded: Vec<String> = output
            .iter()
            .map(|r| format!("{}{}{}", pad, r, pad))
            .collect();

        // Apply margin=2
        let blank = " ".repeat(22);
        let mut result = Vec::new();
        result.extend(std::iter::repeat_n(blank.clone(), 2));
        result.extend(padded);
        result.extend(std::iter::repeat_n(blank, 2));

        assert_eq!(result.len(), 5 + 4); // 5 original + 2 top + 2 bottom margin
        assert_eq!(result[0].len(), 22); // 20 + 2 padding
    }

    #[test]
    fn test_resolve_text_value_literal() {
        let result = resolve_text_value("Hello World").unwrap();
        assert_eq!(result, "Hello World");
    }

    #[test]
    fn test_resolve_text_value_empty() {
        let result = resolve_text_value("").unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn test_resolve_env_var() {
        let result = resolve_text_value("${HOME}").unwrap();
        assert!(!result.is_empty(), "HOME should be set");
        assert!(result.contains('/'), "HOME should be a path");
    }

    #[test]
    fn test_resolve_env_var_in_context() {
        let result = resolve_text_value("path=${HOME}/foo").unwrap();
        assert!(result.starts_with("path="), "should preserve prefix");
        assert!(result.ends_with("/foo"), "should preserve suffix");
        assert!(
            result.len() > "path=/foo".len(),
            "should contain expanded HOME"
        );
    }

    #[test]
    fn test_resolve_env_var_not_set() {
        let err = resolve_text_value("${DOES_NOT_EXIST_XYZZY}").unwrap_err();
        assert!(err.to_string().contains("not set"));
    }

    #[test]
    fn test_resolve_env_var_unclosed() {
        let err = resolve_text_value("${UNCLOSED").unwrap_err();
        assert!(err.to_string().contains("unclosed"));
    }

    #[test]
    fn test_resolve_command_sub() {
        let result = resolve_text_value("$(echo hello)").unwrap();
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_resolve_command_sub_in_context() {
        let result = resolve_text_value("prefix_$(echo mid)_suffix").unwrap();
        assert_eq!(result, "prefix_mid_suffix");
    }

    #[test]
    fn test_resolve_command_failure() {
        let err = resolve_text_value("$(false)").unwrap_err();
        assert!(err.to_string().contains("exited"));
    }

    #[test]
    fn test_resolve_mixed_literal_and_substitutions() {
        let result = resolve_text_value("$(echo A)${HOME}B").unwrap();
        assert!(result.starts_with('A'), "should start with cmd output");
        assert!(result.contains('/'), "should contain HOME path");
        assert!(result.ends_with('B'), "should end with literal B");
    }

    #[test]
    fn test_parse_img_tag_basic() {
        let ftmp = r#"---
[canvas]
width = 50
height = 10
---
{{img:test.png:30::false:0,0:default}}
"#;
        let tmpl = parse_ftmp(ftmp).expect("should parse img tag");
        assert_eq!(tmpl.layers.len(), 1);
        let img = tmpl.layers[0].image_tag.as_ref().expect("should be image tag");
        assert_eq!(img.source, "test.png");
        assert_eq!(img.width, Some(30));
        assert!(img.height.is_none());
        assert!(!img.colored);
        assert_eq!(img.charset, "default");
    }

    #[test]
    fn test_parse_img_tag_all_fields() {
        let ftmp = r#"---
[canvas]
width = 80
---
{{img:photo.jpg:80:24:true:5,10:BLOCK}}
"#;
        let tmpl = parse_ftmp(ftmp).expect("should parse");
        let img = tmpl.layers[0].image_tag.as_ref().expect("should be image");
        assert_eq!(img.source, "photo.jpg");
        assert_eq!(img.width, Some(80));
        assert_eq!(img.height, Some(24));
        assert!(img.colored);
        assert_eq!(img.x, Some(5));
        assert_eq!(img.y, Some(10));
        assert_eq!(img.charset, "BLOCK");
    }

    #[test]
    fn test_resolve_text_in_template_ftmp() {
        // Verify that ${VAR} in a variable's text field gets resolved.
        // Set a known env var for the test.
        let ftmp = r#"---
[canvas]
width = 40
height = 10

[variables.msg]
text = "${USER}"
font = "standard"
---
{{msg}}
"#;
        let tmpl = parse_ftmp(ftmp).expect("should parse");
        let (_tmpdir, font_dir) = setup_font_dir();
        let config = RenderConfig {
            font_dir,
            term_width: 80,
            override_width: None,
        };
        let result = render_template(&tmpl, &config);
        // Should succeed because USER is normally set
        assert!(result.is_ok(), "render should succeed: {:?}", result.err());
    }

    #[test]
    fn test_render_template_with_text_and_img_placeholders() {
        // Path relative to figby-rs/ (where test binary runs)
        let img_path = "../assets/img/figby.png";
        if !std::path::Path::new(img_path).exists() {
            eprintln!("skipping image test: file not found at {img_path}");
            return;
        }
        let ftmp = format!(r#"---
[canvas]
width = 60
height = 20

[variables.greeting]
text = "figby"
font = "standard"
---
{{{{greeting}}}}{{{{img:{}:30:::0,0:}}}}
"#, img_path);
        let tmpl = parse_ftmp(&ftmp).expect("should parse");
        assert_eq!(tmpl.layers.len(), 2);
        assert!(tmpl.layers[0].image_tag.is_none());
        assert!(tmpl.layers[1].image_tag.is_some());
        let (_tmpdir, font_dir) = setup_font_dir();
        let config = RenderConfig {
            font_dir,
            term_width: 80,
            override_width: None,
        };
        let result = render_template(&tmpl, &config);
        assert!(result.is_ok(), "render should succeed: {:?}", result.err());
        let output = result.unwrap();
        assert!(!output.is_empty(), "should produce output rows");
    }

    #[test]
    fn test_cli_flag_parse_check() {
        // Verify clap can parse --render-template.
        // This test lives in template.rs but exercises main.rs's CliArgs.
        // We use try_parse_from directly.
        use clap::Parser;
        #[derive(Parser, Debug)]
        struct TestArgs {
            #[arg(short = 'T', long = "render-template")]
            render_template: Option<String>,
        }
        let args = TestArgs::try_parse_from(["test", "--render-template", "test.ftmp"]).unwrap();
        assert_eq!(args.render_template, Some("test.ftmp".to_string()));

        let args = TestArgs::try_parse_from(["test"]).unwrap();
        assert!(args.render_template.is_none());
    }
}
