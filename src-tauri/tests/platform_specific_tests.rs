#[cfg(test)]
mod platform_specific_tests {
    use std::env;
    use std::path::Path;
    
    // Test platform-specific path handling
    #[test]
    fn test_platform_path_handling() {
        // Get the home directory in a platform-specific way
        let home_dir = env::var("HOME").or_else(|_| env::var("USERPROFILE")).expect("Could not find home directory");
        let home_path = Path::new(&home_dir);
        
        // Verify the home directory exists
        assert!(home_path.exists(), "Home directory does not exist");
        assert!(home_path.is_dir(), "Home path is not a directory");
        
        // Test platform-specific paths
        #[cfg(windows)]
        {
            // Windows-specific paths
            let windows_path = Path::new("C:\\Windows");
            if windows_path.exists() {
                assert!(windows_path.is_dir(), "Windows directory is not a directory");
            }
            
            // Test Windows UNC paths if possible
            let unc_path = Path::new("\\\\localhost\\c$");
            if unc_path.exists() {
                assert!(unc_path.is_dir(), "UNC path is not a directory");
            }
        }
        
        #[cfg(target_os = "macos")]
        {
            // macOS-specific paths
            let applications_path = Path::new("/Applications");
            assert!(applications_path.exists(), "Applications directory does not exist");
            assert!(applications_path.is_dir(), "Applications path is not a directory");
            
            // Test macOS-specific directories
            let library_path = home_path.join("Library");
            if library_path.exists() {
                assert!(library_path.is_dir(), "Library directory is not a directory");
            }
        }
        
        #[cfg(target_os = "linux")]
        {
            // Linux-specific paths
            let etc_path = Path::new("/etc");
            assert!(etc_path.exists(), "etc directory does not exist");
            assert!(etc_path.is_dir(), "etc path is not a directory");
            
            // Test Linux-specific directories
            let usr_path = Path::new("/usr");
            assert!(usr_path.exists(), "usr directory does not exist");
            assert!(usr_path.is_dir(), "usr path is not a directory");
        }
    }
    
    // Test platform-specific environment variables
    #[test]
    fn test_platform_environment_variables() {
        // Test common environment variables that should exist on all platforms
        assert!(env::var("PATH").is_ok(), "PATH environment variable not found");
        
        // Platform-specific environment variables
        #[cfg(windows)]
        {
            // Windows-specific environment variables
            assert!(env::var("SYSTEMROOT").is_ok() || env::var("windir").is_ok(), 
                "Windows system directory environment variable not found");
            assert!(env::var("TEMP").is_ok() || env::var("TMP").is_ok(), 
                "Windows temp directory environment variable not found");
        }
        
        #[cfg(target_os = "macos")]
        {
            // macOS-specific environment variables
            // TMPDIR is used on macOS for temporary directory
            assert!(env::var("TMPDIR").is_ok(), "TMPDIR environment variable not found");
        }
        
        #[cfg(target_os = "linux")]
        {
            // Linux-specific environment variables
            // XDG variables are common on Linux
            let has_xdg = env::var("XDG_CONFIG_HOME").is_ok() || 
                          env::var("XDG_DATA_HOME").is_ok() || 
                          env::var("XDG_CACHE_HOME").is_ok();
            
            // Not all Linux distributions set XDG variables, so this is not a hard failure
            if !has_xdg {
                println!("Note: No XDG environment variables found. This is not an error but might indicate a non-standard Linux environment.");
            }
        }
    }
    
    // Test platform-specific file operations
    #[test]
    fn test_platform_file_operations() {
        use std::fs;
        
        // Create a temporary directory for testing
        let temp_dir = env::temp_dir().join("rise_platform_test");
        let _ = fs::create_dir_all(&temp_dir);
        
        // Platform-specific file operations
        #[cfg(windows)]
        {
            // Test Windows-specific file attributes
            use std::os::windows::fs::MetadataExt;
            
            let test_file = temp_dir.join("windows_test.txt");
            fs::write(&test_file, "Windows test").expect("Failed to write test file");
            
            let metadata = fs::metadata(&test_file).expect("Failed to get metadata");
            
            // Check file attributes (FILE_ATTRIBUTE_ARCHIVE is 0x20)
            assert!(metadata.file_attributes() & 0x20 > 0, "File should have archive attribute");
            
            // Clean up
            let _ = fs::remove_file(&test_file);
        }
        
        #[cfg(unix)]
        {
            // Test Unix-specific file permissions
            use std::os::unix::fs::PermissionsExt;
            
            let test_file = temp_dir.join("unix_test.txt");
            fs::write(&test_file, "Unix test").expect("Failed to write test file");
            
            let metadata = fs::metadata(&test_file).expect("Failed to get metadata");
            let permissions = metadata.permissions();
            
            // Check that the file has some permissions
            assert!(permissions.mode() > 0, "File should have permissions");
            
            // Clean up
            let _ = fs::remove_file(&test_file);
        }
        
        // Clean up the temporary directory
        let _ = fs::remove_dir_all(&temp_dir);
    }
    
    // Test platform-specific executable extensions
    #[test]
    fn test_platform_executable_extensions() {
        use std::process::Command;
        
        #[cfg(windows)]
        {
            // On Windows, executables have .exe extension
            let output = Command::new("where")
                .arg("cmd.exe")
                .output();
                
            assert!(output.is_ok(), "Failed to locate cmd.exe");
            let output = output.unwrap();
            assert!(output.status.success(), "where command failed");
            
            let stdout = String::from_utf8_lossy(&output.stdout);
            assert!(stdout.contains(".exe"), "Windows executable should have .exe extension");
        }
        
        #[cfg(unix)]
        {
            // On Unix, executables don't need extensions
            let output = Command::new("which")
                .arg("ls")
                .output();
                
            assert!(output.is_ok(), "Failed to locate ls");
            let output = output.unwrap();
            assert!(output.status.success(), "which command failed");
            
            let stdout = String::from_utf8_lossy(&output.stdout);
            assert!(!stdout.is_empty(), "Should find ls executable");
        }
    }
    
    // Test platform-specific line endings
    #[test]
    fn test_platform_line_endings() {
        use std::fs;
        
        // Create a temporary directory for testing
        let temp_dir = env::temp_dir().join("rise_line_ending_test");
        let _ = fs::create_dir_all(&temp_dir);
        
        // Create a test file with platform-specific line endings
        let test_file = temp_dir.join("line_endings.txt");
        
        #[cfg(windows)]
        {
            // Windows uses CRLF line endings
            fs::write(&test_file, "Line 1\r\nLine 2\r\nLine 3").expect("Failed to write test file");
            
            let content = fs::read_to_string(&test_file).expect("Failed to read test file");
            assert!(content.contains("\r\n"), "Windows file should contain CRLF line endings");
        }
        
        #[cfg(unix)]
        {
            // Unix uses LF line endings
            fs::write(&test_file, "Line 1\nLine 2\nLine 3").expect("Failed to write test file");
            
            let content = fs::read_to_string(&test_file).expect("Failed to read test file");
            assert!(content.contains("\n") && !content.contains("\r\n"), 
                "Unix file should contain LF line endings and not CRLF");
        }
        
        // Clean up
        let _ = fs::remove_file(&test_file);
        let _ = fs::remove_dir_all(&temp_dir);
    }
}