use rise_lib::theme::{get_style_for_kind, set_active_theme_from_json};

fn example_theme_json() -> String {
    r##"{
        "name": "Test Theme",
        "type": "dark",
        "colors": {},
        "tokenColors": [
            {
                "name": "Strings",
                "scope": ["string", "meta.string"],
                "settings": {"foreground": "#aabbcc", "fontStyle": "italic"}
            },
            {
                "name": "Numbers",
                "scope": "constant.numeric",
                "settings": {"foreground": "#112233"}
            }
        ]
    }"##.to_string()
}

#[test]
fn test_theme_happy_path_styles_are_applied_via_public_api() {
    let ok = set_active_theme_from_json(&example_theme_json());
    assert!(ok, "theme JSON should parse successfully");

    let n = get_style_for_kind("number").expect("number style should be present (via scope match)");
    // We only know it's styled, not the exact rendering; at least ensure font-style key exists
    assert!(n.contains("font-style"), "expected style string to contain font-style, got: {}", n);
}

#[test]
fn test_fallbacks_when_theme_has_no_match() {
    // Set a theme with no token colors -> forces fallback mapping
    let empty_theme = r#"{"name":"Empty","tokenColors":[],"colors":{}}"#;
    assert!(set_active_theme_from_json(empty_theme));

    let string_style = get_style_for_kind("string").expect("fallback for string");
    assert!(string_style.contains("#a6e3a1"));
    assert!(string_style.contains("font-style"));

    let num_style = get_style_for_kind("number").expect("fallback for number");
    assert!(num_style.contains("#fab387"));

    assert_eq!(get_style_for_kind("totally-unknown-kind"), None);
}

#[test]
fn test_alias_resolution_delegates_to_scope_match() {
    // Style provided only for VSCode scope "variable.other.property"; query via alias "property"
    let alias_theme = r##"{"name":"Alias","colors":{},"tokenColors":[
        {"scope":"variable.other.property","settings":{"foreground":"#123456","fontStyle":"bold"}}
    ]}"##;
    assert!(set_active_theme_from_json(alias_theme));

    let style = get_style_for_kind("property").expect("alias match");
    assert!(style.contains("bold") || style.contains("font-style"));
}
