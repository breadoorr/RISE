// Theme schema taken from https://github.com/catppuccin/vscode

use serde::Deserialize;
use serde_json::{Map, Value};
use lazy_static::lazy_static;
use std::fs;
use std::sync::RwLock;
use crate::commands;

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


fn load_theme_from_config() -> Theme {
    // Determine theme file from config; fallback to default (frappe)
    let theme_name = commands::get_app_theme();
    let base = env!("CARGO_MANIFEST_DIR");
    let default_path = format!("{}/src/theme/frappe.json", base);
    let selected_path = if theme_name != "default" {
        println!("Selected theme: {}", theme_name);
        format!("{}/src/theme/{}.json", base, theme_name)
    } else {
        default_path.clone()
    };

    // Read selected theme, with graceful fallback to default
    let contents = fs::read_to_string(&selected_path)
        .or_else(|_| fs::read_to_string(&default_path))
        .unwrap_or_else(|_| "{\n  \"tokenColors\": []\n}".to_string());

    parse_vscode_theme_str(contents.as_str()).unwrap_or_else(|_| Theme { name: "fallback".into(), token_styles: Vec::new() })
}

lazy_static! {
    static ref ACTIVE_THEME: RwLock<Theme> = RwLock::new(load_theme_from_config());
}

/// Reload theme from config at runtime (e.g., call on app start or when user changes theme)
pub fn reload_theme() {
    let mut t = ACTIVE_THEME.write().expect("theme lock poisoned");
    *t = load_theme_from_config();
}

// Public API: get the formatted style for a given kind/scope
pub fn get_style_for_kind(kind: &str) -> Option<String> {
    fn style_from_scope(scope: &str) -> Option<String> {
        let scope_lc = scope.to_lowercase();
        let mut best: Option<&TokenStyle> = None;
        let mut best_score: usize = 0;
        let theme = ACTIVE_THEME.read().expect("theme lock poisoned");
        for ts in &theme.token_styles {
            for sc in &ts.scope {
                let s = sc.to_lowercase();
                let mut score = 0;
                if scope_lc == s { score = s.len(); }
                else if scope_lc.ends_with(&s) { score = s.len(); }
                else if s.ends_with(&scope_lc) { score = scope_lc.len(); }
                else if scope_lc.contains(&s) { score = s.len() / 2; }
                else if s.contains(&scope_lc) { score = scope_lc.len() / 2; }

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

    // 1) Try direct match first
    if let Some(s) = style_from_scope(kind) { return Some(s); }

    // 2) Try well-known aliases (esp. for JSON/tree-sitter captures)
    // Map tree-sitter capture kinds to common VSCode scopes
    let kind_lc = kind.to_lowercase();
    let aliases: &[&str] = match kind_lc.as_str() {
        // generic programming
        "string" => &["string", "meta.string"],
        "number" | "numeric" => &["constant.numeric", "number"],
        "boolean" | "true" | "false" => &["constant.language.boolean", "keyword.operator", "boolean"],
        "null" => &["constant.language.null", "constant.language"],
        "property" | "pair_key" | "object_key" => &["variable.other.property", "meta.object.property", "property"],
        "escape" | "escape_sequence" => &["constant.character.escape", "string"],
        "punctuation.bracket" | "bracket" => &["punctuation.bracket", "punctuation"],
        "punctuation.delimiter" | "delimiter" | "separator" | ":" | "," => &["punctuation.separator", "punctuation"],
        "operator" => &["keyword.operator"],
        // Try some broader fallbacks
        _ => &[],
    };

    for a in aliases { if let Some(s) = style_from_scope(a) { return Some(s); } }

    // 3) Final fallback: minimal default colors for common kinds if theme had nothing
    match kind_lc.as_str() {
        "string" => Some("color: #a6e3a1;   font-style: none".into()),
        "number" | "numeric" => Some("color: #fab387;   font-style: none".into()),
        "boolean" | "true" | "false" => Some("color: #f9e2af;   font-style: none".into()),
        "null" => Some("color: #f38ba8;   font-style: italic".into()),
        "property" | "pair_key" | "object_key" => Some("color: #89b4fa;   font-style: none".into()),
        "punctuation.bracket" | "punctuation.delimiter" | "delimiter" | "separator" => Some("color: #9399b2;   font-style: none".into()),
        _ => None,
    }
}
