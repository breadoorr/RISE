use std::collections::{HashMap, HashSet};
use std::fs;
use std::sync::Mutex;
use lazy_static::lazy_static;
use serde::Serialize;
use streaming_iterator::StreamingIterator;
use tree_sitter::{Language, Parser, Point, Query, QueryCursor, QueryMatches, Tree};


lazy_static! {
    static ref PARSERS: Mutex<HashMap<String, Parser>> = Mutex::new(HashMap::new());
    static ref QUERIES: Mutex<HashMap<String, Query>> = Mutex::new(HashMap::new());
    static ref TREES: Mutex<HashMap<String, Tree>> = Mutex::new(HashMap::new());
    static ref FILE_CONTENTS: Mutex<HashMap<String, String>> = Mutex::new(HashMap::new());
    // Cache of highlighted token spans per file path: (start_byte, end_byte, scope)
    static ref CACHED_SPANS: Mutex<HashMap<String, Vec<(usize, usize, String)>>> = Mutex::new(HashMap::new());
}

pub fn get_language_object(language: &str) -> Language {
    match language {
        "rust" => tree_sitter_rust::LANGUAGE.into(),
        "python" => tree_sitter_python::LANGUAGE.into(),
        "c" => tree_sitter_c::LANGUAGE.into(),
        "java" => tree_sitter_java::LANGUAGE.into(),
        "c_sharp" => tree_sitter_c_sharp::LANGUAGE.into(),
        "sequel" => tree_sitter_sequel::LANGUAGE.into(),
        "json" => tree_sitter_json::LANGUAGE.into(),
        _ => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
    }
}

pub fn get_language_query(language: &str) -> &str {
    match language {
        "rust" => tree_sitter_rust::HIGHLIGHTS_QUERY.into(),
        "python" => tree_sitter_python::HIGHLIGHTS_QUERY.into(),
        "c" => tree_sitter_c::HIGHLIGHT_QUERY.into(),
        "java" => tree_sitter_java::HIGHLIGHTS_QUERY.into(),
        "c_sharp" => tree_sitter_c_sharp::NODE_TYPES.into(),
        "sequel" => tree_sitter_sequel::HIGHLIGHTS_QUERY.into(),
        "json" => tree_sitter_json::HIGHLIGHTS_QUERY.into(),
        _ => tree_sitter_javascript::HIGHLIGHT_QUERY.into(),
    }
}

pub fn collect(node: tree_sitter::Node, _src: &str, out: &mut Vec<(usize, usize, String)>) {
    if node.child_count() == 0 {
        let kind = node.kind().to_string();
        // println!("{}", kind);
        out.push((node.start_byte(), node.end_byte(), kind));
    } else {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            collect(child, _src, out);
        }
    }
}

pub fn escape_html(text: &str) -> String {
    text.replace("&", "&amp;")
        .replace("<", "&lt;")
        .replace(">", "&gt;")
}

pub fn calculate_edit(old_code: &str, new_code: &str) -> Option<tree_sitter::InputEdit> {
    let old_bytes = old_code.as_bytes();
    let new_bytes = new_code.as_bytes();

    let mut start = 0;
    let min_len = old_bytes.len().min(new_bytes.len());

    while start < min_len && old_bytes[start] == new_bytes[start] {
        start += 1;
    }

    let mut old_end = old_bytes.len();
    let mut new_end = new_bytes.len();

    while old_end > start && new_end > start && old_bytes[old_end - 1] == new_bytes[new_end - 1] {
        old_end -= 1;
        new_end -= 1;
    }

    if start == old_end && start == new_end {
        return None;
    }

    Some(tree_sitter::InputEdit {
        start_byte: start,
        old_end_byte: old_end,
        new_end_byte: new_end,
        start_position: byte_to_point(old_code, start),
        old_end_position: byte_to_point(old_code, old_end),
        new_end_position: byte_to_point(new_code, new_end),
    })
}

fn byte_to_point(text: &str, byte_offset: usize) -> Point {
    let mut row = 0;
    let mut last_newline_byte = 0;
    for (i, byte) in text.bytes().enumerate() {
        if i >= byte_offset {
            break;
        }
        if byte == b'\n' {
            row += 1;
            last_newline_byte = i + 1;
        }
    }
    let column = byte_offset - last_newline_byte;
    Point::new(row, column)
}

fn prev_line_start(text: &str, byte_idx: usize) -> usize {
    let bytes = text.as_bytes();
    let mut i = byte_idx.min(bytes.len());
    while i > 0 {
        if bytes[i - 1] == b'\n' { break; }
        i -= 1;
    }
    i
}

fn next_line_end(text: &str, byte_idx: usize) -> usize {
    let bytes = text.as_bytes();
    let mut i = byte_idx.min(bytes.len());
    while i < bytes.len() {
        if bytes[i] == b'\n' { i += 1; break; }
        i += 1;
    }
    i
}

#[derive(Serialize)]
pub struct HighlightResult {
    pub html: String,
}

pub fn find_matches<'a>(code: String, matches: &mut QueryMatches<&'a[u8], &'a[u8]>, query: &Query, seen: &mut HashSet<(usize, usize)>, final_spans: &mut Vec<(usize, usize, String)>) {
    while let Some(mat) = matches.next() {
        for cap in mat.captures {
            let node = cap.node;
            // Include all captured nodes, not just leaves. JSON strings and keys are often non-leaf nodes.
            let start = node.start_byte();
            let end = node.end_byte();
            if start < end && end <= code.len() && seen.insert((start, end)) {
                let scope = query.capture_names()[cap.index as usize].to_string();
                final_spans.push((start, end, scope));
            }
        }
    }
}

fn normalize_spans(mut spans: Vec<(usize, usize, String)>) -> Vec<(usize, usize, String)> {
    if spans.is_empty() { return spans; }
    spans.sort_by(|a, b| {
        if a.0 != b.0 { a.0.cmp(&b.0) } else { b.1.cmp(&a.1) }
    });
    let mut result: Vec<(usize, usize, String)> = Vec::with_capacity(spans.len());
    let mut current_end: usize = 0;
    for (s, e, k) in spans.into_iter() {
        if e <= s { continue; }
        if e <= current_end { continue; }
        let start = s.max(current_end);
        if start >= e { continue; }
        result.push((start, e, k));
        current_end = e;
    }
    result
}

#[tauri::command]
pub fn highlight_ast(code: String, language: String, path: String) -> Result<Vec<(usize, usize, String)>, String> {
    let mut parsers = PARSERS.lock().unwrap();
    let mut queries = QUERIES.lock().unwrap();
    let mut trees = TREES.lock().unwrap();
    let mut file_contents = FILE_CONTENTS.lock().unwrap();
    if !parsers.contains_key(&language){
        let mut parser = Parser::new();
        let lang = get_language_object(&language);
        parser.set_language(&lang).map_err(|e| e.to_string())?;
        parsers.insert(language.clone(), parser);
    }

    if !queries.contains_key(&language){
        let query = Query::new(&get_language_object(&language), get_language_query(&language)).expect("query error");
        queries.insert(language.clone(), query);
    }
    let old_code = file_contents.get(&path).cloned().unwrap_or_default();
    let edit = calculate_edit(&old_code, &code);

    if let Some(tree) = trees.get_mut(&path) {
        if let Some(ref edit) = edit {
            tree.edit(edit);
        }
    }

    let parser = parsers.get_mut(&language).unwrap();
    let old_tree_ref = trees.get(&path);
    let new_tree = parser.parse(&code, old_tree_ref);
    if let Some(tree) = new_tree {
        let query = queries.get(&language).unwrap();
        let mut cursor = QueryCursor::new();

        let mut spans_cache = CACHED_SPANS.lock().unwrap();
        let mut full_recompute = false;
        let old_code = file_contents.get(&path).cloned().unwrap_or_default();
        let edit_opt = calculate_edit(&old_code, &code);
        if !spans_cache.contains_key(&path) || edit_opt.is_none() {
            full_recompute = true;
        }

        let mut final_spans: Vec<(usize, usize, String)> = Vec::new();

        if full_recompute {
            let mut seen: HashSet<(usize, usize)> = HashSet::new();
            let mut matches = cursor.matches(&query, tree.root_node(), code.as_bytes());
            find_matches(code.clone(), &mut matches, query, &mut seen, &mut final_spans);
            final_spans.sort_by_key(|(s, _e, _k)| *s);
            final_spans = normalize_spans(final_spans);
            spans_cache.insert(path.clone(), final_spans.clone());
        } else {
            // Incremental path
            let edit = edit_opt.unwrap();
            let old_start = edit.start_byte;
            let old_end = edit.old_end_byte;
            let new_end = edit.new_end_byte;
            let diff: isize = new_end as isize - old_end as isize;

            let range_start = prev_line_start(&code, edit.start_byte);
            let range_end = next_line_end(&code, edit.new_end_byte);

            cursor.set_byte_range(range_start..range_end);
            let mut seen: HashSet<(usize, usize)> = HashSet::new();
            let mut matches = cursor.matches(&query, tree.root_node(), code.as_bytes());
            let mut new_region_spans: Vec<(usize, usize, String)> = Vec::new();
            find_matches(code.clone(), &mut matches, query, &mut seen, &mut new_region_spans);
            new_region_spans.sort_by_key(|(s, _e, _k)| *s);

            let mut merged: Vec<(usize, usize, String)> = Vec::new();
            if let Some(existing) = spans_cache.get(&path) {
                for (s, e, k) in existing.iter().cloned() {
                    if e <= old_start {
                        // before edit region: keep as-is
                        merged.push((s, e, k));
                    } else if s >= old_end {
                        // after edit region: shift by diff
                        let ns = (s as isize + diff) as usize;
                        let ne = (e as isize + diff) as usize;
                        merged.push((ns, ne, k));
                    }
                }
            }

            // Insert newly computed spans for the edited region (already in new coordinates)
            merged.extend(new_region_spans.into_iter());
            merged.sort_by_key(|(s, _e, _k)| *s);

            // Normalize to avoid overlaps and duplicates
            let normalized = normalize_spans(merged);
            spans_cache.insert(path.clone(), normalized.clone());
            final_spans = normalized;
        }

        trees.insert(path.clone(), tree);
        file_contents.insert(path.clone(), code);
        Ok(spans_cache.get(&path).cloned().unwrap_or(final_spans))
    } else {
        Ok(vec![])
    }
}

#[tauri::command]
pub fn highlight_html(
    language: String,
    matches: Vec<usize>,
    query_len: usize,
    path: String,
) -> String {
    let code: String = {
        if let Ok(map) = crate::commands::EDITOR_BUFFERS.lock() {
            if let Some(buf) = map.get(&path) {
                buf.content.clone()
            } else {
                fs::read_to_string(&path).unwrap_or_default()
            }
        } else {
            fs::read_to_string(&path).unwrap_or_default()
        }
    };

    if code.is_empty() {
        return String::new();
    }
    if code.len() > 500_000 {
        return escape_html(&code);
    }

    let spans = match highlight_ast(code.clone(), language, path) {
        Ok(spans) => spans,
        Err(_) => return escape_html(&code),
    };

    let mut html = String::with_capacity(code.len() * 2);
    let mut last_index: usize = 0;

    let match_set: HashSet<(usize, usize)> = if query_len > 0 {
        matches.into_iter().map(|m| (m, m + query_len)).collect()
    } else {
        HashSet::new()
    };

    let mut spans_sorted = spans;
    spans_sorted.sort_by_key(|(s, _e, _k)| *s);

    for (start, end, kind) in spans_sorted.into_iter() {
        if start > last_index {
            let plain = &code[last_index..start];
            html.push_str(&escape_html(plain));
        }
        if end <= code.len() && start < end {
            let s = start.max(last_index);
            if s < end {
                let raw = &code[s..end];
                let escaped = escape_html(raw);
                let is_match = match_set.contains(&(start, end));
                let style_opt = crate::theme::get_style_for_kind(&kind);
                if let Some(style) = style_opt {
                    if is_match {
                        // html.push_str(&format!("<span class=\"token find-match\" style=\"{}\">{}</span>", style, escaped));
                    } else {
                        println!("{:?}", style.clone());
                        html.push_str(&format!("<span class=\"token\" style=\"{}\">{}</span>", style, escaped));
                        // html.push_str(&escaped);
                    }
                } else {
                    if is_match {
                        html.push_str(&format!("<span class=\"token find-match\">{}</span>", escaped));
                    } else {
                        // No style and not a match: emit plain text to reduce unnecessary spans
                        html.push_str(&escaped);
                    }
                }
            }
        }
        last_index = end.min(code.len());
    }

    if last_index < code.len() {
        html.push_str(&escape_html(&code[last_index..]));
    }

    html.replace("\n\n", "\n<span class=\"empty-line\"> </span>\n")
}