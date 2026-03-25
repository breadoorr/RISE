use std::fs;
use std::path::Path;

// These UI compatibility tests validate the current Tauri v2 setup by
// inspecting the Rust source (no building, no UI spawning). This keeps
// them fast, deterministic, and cross-platform.

fn lib_rs_path() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join("lib.rs")
}

fn read_lib_rs() -> String {
    fs::read_to_string(lib_rs_path()).expect("Failed to read src/lib.rs")
}

#[test]
fn test_plugins_enabled_in_builder() {
    let src = read_lib_rs();
    // Verify plugins are registered
    assert!(src.contains(".plugin(tauri_plugin_dialog::init())"),
        "Expected dialog plugin to be initialized in Builder");
    assert!(src.contains(".plugin(tauri_plugin_fs::init())"),
        "Expected fs plugin to be initialized in Builder");
}

#[test]
fn test_expected_commands_are_registered() {
    let src = read_lib_rs();

    // The invoke handler is built via tauri::generate_handler![ ... ]
    // We check for a few representative commands that the UI depends on.
    let must_have = [
        "get_actions",
        "perform_action",
        "get_app_theme",
        "update_app_theme",
        "search_in_project",
        "search_paths_in_project",
        "replace_in_project",
        // Process/terminal related
        "start_process",
        "write_to_process",
        "kill_process",
    ];

    // First ensure we even have the handler macro around
    assert!(src.contains("tauri::generate_handler!["),
        "Expected tauri::generate_handler! macro in src/lib.rs");

    for sym in must_have.iter() {
        assert!(src.contains(sym), "Expected command '{}' to be registered in generate_handler!", sym);
    }
}

#[test]
fn test_macos_window_style_configuration_present() {
    let src = read_lib_rs();
    // Check that we have macOS-specific configuration behind cfg gates
    let has_cfg = src.contains("cfg(target_os = \"macos\")");
    let has_transparent = src.contains("TitleBarStyle::Transparent");
    let has_cocoa_bits = src.contains("NSFullSizeContentViewWindowMask")
        || src.contains("setTitlebarAppearsTransparent_");

    assert!(has_cfg, "Expected macOS cfg-gated code in src/lib.rs");
    assert!(has_transparent, "Expected TitleBarStyle::Transparent on macOS");
    assert!(has_cocoa_bits, "Expected extra macOS window bridging calls present");
}