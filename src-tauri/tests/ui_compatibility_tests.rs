// #[cfg(test)]
// mod ui_compatibility_tests {
//     use std::env;
//     use std::process::Command;
//
//     // Test that the application can be built on the current platform
//     #[test]
//     fn test_build_on_current_platform() {
//         // Skip this test in CI environments as it's handled by the workflow
//         if env::var("CI").is_ok() {
//             return;
//         }
//
//         // Run cargo build to ensure the application can be built
//         let output = Command::new("cargo")
//             .args(["build", "--verbose"])
//             .current_dir(env!("CARGO_MANIFEST_DIR"))
//             .output()
//             .expect("Failed to execute cargo build");
//
//         assert!(output.status.success(),
//             "Build failed: {}", String::from_utf8_lossy(&output.stderr));
//     }
//
//     // Test window creation capabilities
//     #[test]
//     fn test_window_config() {
//         use std::fs;
//         use std::path::Path;
//
//         // Read the tauri.conf.json file to verify window configuration
//         let config_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tauri.conf.json");
//         let config_content = fs::read_to_string(config_path)
//             .expect("Failed to read tauri.conf.json");
//
//         // Check that the window configuration exists
//         assert!(config_content.contains("\"windows\""),
//             "Window configuration not found in tauri.conf.json");
//
//         // Check for platform-specific window properties
//         #[cfg(target_os = "macos")]
//         {
//             // macOS-specific window properties
//             assert!(config_content.contains("\"transparent\"") || config_content.contains("\"decorations\""),
//                 "macOS window properties not found");
//         }
//
//         #[cfg(target_os = "windows")]
//         {
//             // Windows-specific window properties
//             assert!(config_content.contains("\"decorations\""),
//                 "Windows window properties not found");
//         }
//
//         #[cfg(target_os = "linux")]
//         {
//             // Linux-specific window properties
//             assert!(config_content.contains("\"decorations\""),
//                 "Linux window properties not found");
//         }
//     }
//
//     // Test dialog capabilities
//     #[test]
//     fn test_dialog_capabilities() {
//         use std::fs;
//         use std::path::Path;
//
//         // Read the tauri.conf.json file to verify dialog capabilities
//         let config_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tauri.conf.json");
//         let config_content = fs::read_to_string(config_path)
//             .expect("Failed to read tauri.conf.json");
//
//         // Check that dialog capabilities are enabled
//         assert!(config_content.contains("\"dialog\""),
//             "Dialog capabilities not found in tauri.conf.json");
//         assert!(config_content.contains("\"open\"") && config_content.contains("\"save\""),
//             "Open and save dialog capabilities not found");
//     }
//
//     // Test shell capabilities
//     #[test]
//     fn test_shell_capabilities() {
//         use std::fs;
//         use std::path::Path;
//
//         // Read the tauri.conf.json file to verify shell capabilities
//         let config_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tauri.conf.json");
//         let config_content = fs::read_to_string(config_path)
//             .expect("Failed to read tauri.conf.json");
//
//         // Check that shell capabilities are enabled
//         assert!(config_content.contains("\"shell\""),
//             "Shell capabilities not found in tauri.conf.json");
//         assert!(config_content.contains("\"open\""),
//             "Shell open capability not found");
//     }
//
//     // Test bundle configuration
//     #[test]
//     fn test_bundle_configuration() {
//         use std::fs;
//         use std::path::Path;
//
//         // Read the tauri.conf.json file to verify bundle configuration
//         let config_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tauri.conf.json");
//         let config_content = fs::read_to_string(config_path)
//             .expect("Failed to read tauri.conf.json");
//
//         // Check that bundle configuration exists
//         assert!(config_content.contains("\"bundle\""),
//             "Bundle configuration not found in tauri.conf.json");
//
//         // Check for platform-specific icons
//         // assert!(config_content.contains("\"icon.icns\""),
//         //     "macOS icon not found in bundle configuration");
//         // assert!(config_content.contains("\"icon.ico\""),
//         //     "Windows icon not found in bundle configuration");
//         // assert!(config_content.contains("\"128x128.png\""),
//         //     "Linux icon not found in bundle configuration");
//     }
// }