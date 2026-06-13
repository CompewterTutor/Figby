use font_kit::error::FontLoadingError;
use font_kit::error::SelectionError;
use font_kit::font::Font;
use font_kit::handle::Handle;
use font_kit::source::SystemSource;
use std::fmt;

#[derive(Debug)]
pub enum FontGenError {
    Selection(SelectionError),
    FontLoading(FontLoadingError),
}

impl fmt::Display for FontGenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FontGenError::Selection(e) => write!(f, "font selection error: {e}"),
            FontGenError::FontLoading(e) => write!(f, "font loading error: {e}"),
        }
    }
}

impl std::error::Error for FontGenError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FontGenError::Selection(e) => Some(e),
            FontGenError::FontLoading(e) => Some(e),
        }
    }
}

impl From<SelectionError> for FontGenError {
    fn from(e: SelectionError) -> Self {
        FontGenError::Selection(e)
    }
}

impl From<FontLoadingError> for FontGenError {
    fn from(e: FontLoadingError) -> Self {
        FontGenError::FontLoading(e)
    }
}

#[derive(Debug, Clone)]
pub struct FontFamilyInfo {
    pub family: String,
    pub styles: Vec<String>,
}

fn describe_style(font: &Font) -> String {
    let props = font.properties();
    format!(
        "Weight: {}, Style: {:?}",
        props.weight.0 as u32, props.style
    )
}

fn family_is_monospace(name: &str, source: &SystemSource) -> bool {
    if name.to_lowercase().contains("mono") {
        return true;
    }
    if let Ok(family_handle) = source.select_family_by_name(name) {
        let handles = family_handle.fonts();
        if let Some(handle) = handles.first() {
            if let Ok(font) = handle.load() {
                return font.is_monospace();
            }
        }
    }
    false
}

fn load_styles(handles: &[Handle]) -> Vec<String> {
    handles
        .iter()
        .filter_map(|handle| {
            let font = handle.load().ok()?;
            Some(describe_style(&font))
        })
        .collect()
}

pub fn list_system_fonts() -> Result<Vec<FontFamilyInfo>, FontGenError> {
    let source = SystemSource::new();
    let family_names = source.all_families()?;
    let mut result = Vec::with_capacity(family_names.len());

    for name in family_names {
        let styles = match source.select_family_by_name(&name) {
            Ok(family_handle) => load_styles(family_handle.fonts()),
            Err(_) => Vec::new(),
        };
        result.push(FontFamilyInfo {
            family: name,
            styles,
        });
    }

    Ok(result)
}

pub fn list_monospace_fonts() -> Result<Vec<FontFamilyInfo>, FontGenError> {
    let source = SystemSource::new();
    let family_names = source.all_families()?;
    let mut result = Vec::new();

    for name in family_names {
        if family_is_monospace(&name, &source) {
            let styles = match source.select_family_by_name(&name) {
                Ok(family_handle) => load_styles(family_handle.fonts()),
                Err(_) => Vec::new(),
            };
            result.push(FontFamilyInfo {
                family: name,
                styles,
            });
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_system_fonts_not_empty() {
        let fonts = list_system_fonts().expect("system font enumeration should succeed");
        assert!(
            !fonts.is_empty(),
            "at least one font family should be found"
        );
    }

    #[test]
    fn test_list_monospace_fonts_returns_subset() {
        let fonts = list_monospace_fonts().expect("monospace font enumeration should succeed");
        assert!(
            !fonts.is_empty(),
            "at least one monospace font family should be found"
        );
        for info in &fonts {
            let is_mono =
                info.family.to_lowercase().contains("mono") || is_any_font_monospace(&info.family);
            assert!(is_mono, "family '{}' should be monospace", info.family);
        }
    }

    #[test]
    fn test_font_family_info_has_styles() {
        let fonts = list_system_fonts().expect("system font enumeration should succeed");
        let any_with_styles = fonts.iter().any(|f| !f.styles.is_empty());
        assert!(
            any_with_styles,
            "at least one family should have non-empty styles"
        );
    }

    fn is_any_font_monospace(name: &str) -> bool {
        let source = SystemSource::new();
        if let Ok(family_handle) = source.select_family_by_name(name) {
            if let Some(handle) = family_handle.fonts().first() {
                if let Ok(font) = handle.load() {
                    return font.is_monospace();
                }
            }
        }
        false
    }
}
