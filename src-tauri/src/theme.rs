// Theme schema taken from https://github.com/catppuccin/vscode

use serde::Deserialize;
use serde_json::{Map, Value};
use lazy_static::lazy_static;

#[derive(Debug, Default, Clone)]
struct Style {
    color: Option<String>,
    bold: bool,
    italic: bool,
}

#[derive(Debug, Clone)]
struct TokenStyle {
    name: Option<String>,
    scope: Vec<String>,
    style: Style,
}

#[derive (Debug, Clone)]
struct Theme {
    name: String,
    token_styles: Vec<TokenStyle>,
}
#[derive(Deserialize, Debug)]
#[serde (untagged)]
enum VsCodeScope {
    Single(String),
    Multiple(Vec<String>),
}

impl From<VsCodeScope> for Vec<String> {
    fn from(scope: VsCodeScope) -> Self {
        match scope {
            VsCodeScope::Single(s) => vec![s],
            VsCodeScope::Multiple(m) => m,
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde (rename_all = "camelCase")]
struct VsCodeTokenColor {
    name: Option<String>,
    scope: VsCodeScope,
    settings: Map<String, Value>,
}

impl From<VsCodeTokenColor> for TokenStyle {
    fn from(tc: VsCodeTokenColor) -> Self {
        let mut style = Style::default();
        if let Some(foreground) = tc.settings.get("foreground").and_then(|v| v.as_str()) {
            style.color = Some(foreground.to_string());
        }

        if let Some(font_style) = tc.settings.get("fontStyle").and_then(|v| v.as_str()) {
            style.bold = font_style.contains("bold");
            style.italic = font_style.contains("italic");
        }

        Self {
            name: tc.name,
            scope: tc.scope.into(),
            style
        }
    }
}

#[derive (Deserialize, Debug)]
#[serde (rename_all = "camelCase")]
struct VsCodeTheme {
    name: Option<String>,
    #[serde(rename = "type")]
    typ: Option<String>,
    colors: Map<String, Value>,
    token_colors: Vec<VsCodeTokenColor>,
}

fn parse_vscode_theme_str(contents: &str) -> anyhow::Result<Theme> {
    let vscode_theme: VsCodeTheme = serde_json::from_str(contents)?;
    let mut token_styles = Vec::new();

    for token_color in vscode_theme.token_colors {
        token_styles.push(token_color.into());
    }

    Ok(Theme {
        name: vscode_theme.name.unwrap_or_default(),
        token_styles,
    })
}

lazy_static! {
    static ref DEFAULT_THEME: Theme = {
        // Embed theme at compile time to avoid runtime file path issues
        let contents: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/theme/frappe.json"));
        parse_vscode_theme_str(contents).expect("Failed to parse embedded theme")
    };
}

// Public API: get the formatted style for a given kind/scope
pub fn get_style_for_kind(kind: &str) -> Option<String> {
    let kind_lc = kind.to_lowercase();

    // Choose the best matching token style
    let mut best: Option<&TokenStyle> = None;
    let mut best_score: usize = 0;

    for ts in &DEFAULT_THEME.token_styles {
        for scope in &ts.scope {
            let s = scope.to_lowercase();
            let mut score = 0;
            if kind_lc == s { score = s.len(); }
            else if kind_lc.ends_with(&s) { score = s.len(); }
            else if s.ends_with(&kind_lc) { score = kind_lc.len(); }
            else if kind_lc.contains(&s) { score = s.len() / 2; }
            else if s.contains(&kind_lc) { score = kind_lc.len() / 2; }

            if score > best_score {
                best_score = score;
                best = Some(ts);
            }
        }
    }

    if let Some(ts) = best {
        if let Some(color) = &ts.style.color {
            let font_style = if ts.style.italic { "italic" } else if ts.style.bold { "bold" } else { "none" };
            return Some(format!("color: {};   font-style: {}", color, font_style));
        }
    }

    None
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_embedded_theme_style_lookup() {
        // smoke check a few kinds; may or may not match depending on theme contents
        let _ = &*DEFAULT_THEME; // ensure initialized
        let _maybe = get_style_for_kind("keyword");
        // We don't assert on concrete value to avoid coupling to theme, just ensure no panic and format is correct when Some
        if let Some(s) = _maybe { assert!(s.contains("color:")); assert!(s.contains("font-style:")); }
    }
}
