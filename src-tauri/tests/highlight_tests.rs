use rise_lib::highlight::{escape_html, calculate_edit, get_language_object, get_language_query, collect};

#[test]
fn test_escape_html_basic() {
    assert_eq!(escape_html("&"), "&amp;");
    assert_eq!(escape_html("<tag>"), "&lt;tag&gt;");
    assert_eq!(escape_html("a & b < c > d"), "a &amp; b &lt; c &gt; d");
}

#[test]
fn test_calculate_edit_no_change_returns_none() {
    let old = "fn main() {\n    println!(\"hi\");\n}";
    let newc = old;
    assert!(calculate_edit(old, newc).is_none());
}

#[test]
fn test_calculate_edit_prefix_and_suffix_same_middle_changed() {
    let old = "line1\nline2\nline3\nline4\n";
    let newc = "line1\nLINE_TWO\nline3\nline4\n";
    let edit = calculate_edit(old, newc).expect("expected an edit");
    // Start should be at the beginning of second line (after 'line1\n')
    assert!(edit.start_byte > 0);
    // old_end/new_end should move by the size delta of the changed line
    // old "line2\n" (6 bytes) -> new "LINE_TWO\n" (9 bytes)
    assert!(edit.new_end_byte > edit.old_end_byte);
}

#[test]
fn test_language_object_and_query_have_some_content() {
    // smoke tests that we can retrieve a Language and a query string
    let langs = ["rust", "python", "c", "java", "c_sharp", "sequel", "json", "typescript"];
    for lang in langs {
        let _l = get_language_object(lang);
        let q = get_language_query(lang);
        assert!(!q.is_empty(), "Expected non-empty highlight query for {}", lang);
    }
}

#[test]
fn test_collect_leaf_nodes_produces_tokens() {
    let src = "fn main() { let x = 1; }";
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&get_language_object("rust")).unwrap();
    let tree = parser.parse(src, None).unwrap();
    let mut out = Vec::new();
    collect(tree.root_node(), src, &mut out);
    assert!(!out.is_empty(), "Expected some leaf tokens to be collected");
    // ensure spans are within bounds
    for (s, e, _k) in out {
        assert!(s <= e && e <= src.len());
    }
}
