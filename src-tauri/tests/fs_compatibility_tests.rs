#[cfg(test)]
mod fs_compatibility_tests {
    use std::fs;
    use std::path::Path;
    use std::env;

    // Helper function to create a temporary directory for testing
    fn create_temp_dir() -> String {
        let temp_dir = env::temp_dir().join(format!("rise_test_{}", rand::random::<u32>()));
        fs::create_dir_all(&temp_dir).expect("Failed to create temp directory");
        temp_dir.to_string_lossy().to_string()
    }

    // Helper function to clean up after tests
    fn cleanup_temp_dir(path: &str) {
        let _ = fs::remove_dir_all(path);
    }

    #[test]
    fn test_create_directory() {
        let temp_dir = create_temp_dir();
        let test_dir = Path::new(&temp_dir).join("test_dir");
        
        // Test creating a directory
        assert!(!test_dir.exists());
        fs::create_dir_all(&test_dir).expect("Failed to create test directory");
        assert!(test_dir.exists());
        
        cleanup_temp_dir(&temp_dir);
    }

    #[test]
    fn test_write_and_read_file() {
        let temp_dir = create_temp_dir();
        let test_file = Path::new(&temp_dir).join("test_file.txt");
        
        // Test writing to a file
        let test_content = "Hello, cross-platform world!";
        fs::write(&test_file, test_content).expect("Failed to write to test file");
        
        // Test reading from a file
        let read_content = fs::read_to_string(&test_file).expect("Failed to read test file");
        assert_eq!(read_content, test_content);
        
        cleanup_temp_dir(&temp_dir);
    }

    #[test]
    fn test_list_directory_contents() {
        let temp_dir = create_temp_dir();
        
        // Create some files and directories
        let test_dir = Path::new(&temp_dir).join("test_dir");
        fs::create_dir_all(&test_dir).expect("Failed to create test directory");
        
        let test_file1 = Path::new(&temp_dir).join("test_file1.txt");
        fs::write(&test_file1, "File 1").expect("Failed to write to test file 1");
        
        let test_file2 = Path::new(&temp_dir).join("test_file2.txt");
        fs::write(&test_file2, "File 2").expect("Failed to write to test file 2");
        
        // Test listing directory contents
        let entries: Vec<_> = fs::read_dir(&temp_dir)
            .expect("Failed to read directory")
            .filter_map(Result::ok)
            .collect();
        
        assert_eq!(entries.len(), 3); // test_dir, test_file1.txt, test_file2.txt
        
        cleanup_temp_dir(&temp_dir);
    }

    #[test]
    fn test_path_separators() {
        let temp_dir = create_temp_dir();
        
        // Create a nested directory structure using platform-specific path separators
        let nested_dir = Path::new(&temp_dir).join("level1").join("level2").join("level3");
        fs::create_dir_all(&nested_dir).expect("Failed to create nested directories");
        
        // Verify the nested directory exists
        assert!(nested_dir.exists());
        
        // Test with different path separator styles
        #[cfg(windows)]
        {
            // Windows-style backslash paths
            let windows_path = format!("{}\\level1\\level2\\level3", temp_dir);
            assert!(Path::new(&windows_path).exists());
        }
        
        #[cfg(unix)]
        {
            // Unix-style forward slash paths
            let unix_path = format!("{}/level1/level2/level3", temp_dir);
            assert!(Path::new(&unix_path).exists());
        }
        
        cleanup_temp_dir(&temp_dir);
    }

    #[test]
    fn test_file_permissions() {
        let temp_dir = create_temp_dir();
        let test_file = Path::new(&temp_dir).join("permissions_test.txt");
        
        // Create a file
        fs::write(&test_file, "Testing permissions").expect("Failed to write test file");
        
        // Test file permissions (platform-specific)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            
            // Set read-only permissions
            let metadata = fs::metadata(&test_file).expect("Failed to get metadata");
            let mut permissions = metadata.permissions();
            permissions.set_mode(0o444); // read-only for user, group, others
            fs::set_permissions(&test_file, permissions).expect("Failed to set permissions");
            
            // Verify permissions
            let new_metadata = fs::metadata(&test_file).expect("Failed to get metadata");
            let new_permissions = new_metadata.permissions();
            assert_eq!(new_permissions.mode() & 0o777, 0o444);
            
            // Attempt to write should fail
            assert!(fs::write(&test_file, "This should fail").is_err());
            
            // Reset permissions for cleanup
            let mut permissions = new_metadata.permissions();
            permissions.set_mode(0o644); // read-write for user, read for group and others
            fs::set_permissions(&test_file, permissions).expect("Failed to reset permissions");
        }
        
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            
            // On Windows, check if the file has the archive attribute (common for regular files)
            let metadata = fs::metadata(&test_file).expect("Failed to get metadata");
            let attributes = metadata.file_attributes();
            
            // FILE_ATTRIBUTE_ARCHIVE is 0x20
            assert!(attributes & 0x20 > 0);
        }
        
        cleanup_temp_dir(&temp_dir);
    }

    #[test]
    fn test_special_characters_in_filenames() {
        let temp_dir = create_temp_dir();
        
        // Test with special characters that are valid across platforms
        let special_chars = vec![
            "file with spaces.txt",
            "file_with_underscore.txt",
            "file-with-dash.txt",
            "file.with.dots.txt",
            "file_with_numbers_123.txt",
            "UPPERCASE_FILE.txt",
            "lowercase_file.txt",
        ];
        
        for filename in special_chars {
            let file_path = Path::new(&temp_dir).join(filename);
            fs::write(&file_path, "Test content").expect(&format!("Failed to create file: {}", filename));
            assert!(file_path.exists(), "File should exist: {}", filename);
        }
        
        // Platform-specific tests
        #[cfg(windows)]
        {
            // Windows has restrictions on certain characters
            let invalid_chars = vec!["file<with>invalid:chars*.txt", "file|with?invalid\"chars.txt"];
            for filename in invalid_chars {
                let file_path = Path::new(&temp_dir).join(filename);
                assert!(fs::write(&file_path, "Test content").is_err(), 
                    "Creating file with invalid characters should fail: {}", filename);
            }
        }
        
        #[cfg(unix)]
        {
            // Unix allows more special characters, but avoid : for macOS compatibility
            let valid_chars = vec!["file!with@special#chars.txt", "file$with%special^chars.txt"];
            for filename in valid_chars {
                let file_path = Path::new(&temp_dir).join(filename);
                fs::write(&file_path, "Test content").expect(&format!("Failed to create file: {}", filename));
                assert!(file_path.exists(), "File should exist: {}", filename);
            }
        }
        
        cleanup_temp_dir(&temp_dir);
    }
}