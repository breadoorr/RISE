# Cross-Platform Compatibility Tests

This directory contains tests that verify the cross-platform compatibility of the RISE application. These tests are designed to run on all supported platforms (Windows, macOS, and Linux) to ensure that the application works consistently across different operating systems.

## Test Categories

The tests are organized into three main categories:

### 1. File System Compatibility Tests (`fs_compatibility_tests.rs`)

These tests verify that file system operations work correctly across different platforms:

- Creating directories
- Reading and writing files
- Listing directory contents
- Path separators (Windows vs. Unix)
- File permissions
- Special characters in filenames

### 2. UI Compatibility Tests (`ui_compatibility_tests.rs`)

These tests verify that UI-related functionality works correctly across platforms:

- Building the application
- Window configuration
- Dialog capabilities
- Shell capabilities
- Bundle configuration

### 3. Platform-Specific Tests (`platform_specific_tests.rs`)

These tests verify platform-specific behavior:

- Path handling
- Environment variables
- File operations
- Executable extensions
- Line endings

## Running the Tests

You can run all tests with:

```bash
cd src-tauri
cargo test
```

To run a specific test category:

```bash
cargo test --test fs_compatibility_tests
cargo test --test ui_compatibility_tests
cargo test --test platform_specific_tests
```

To run a specific test:

```bash
cargo test test_create_directory
```

## CI/CD Integration

These tests are automatically run as part of the GitHub Actions workflow defined in `.github/workflows/dev-to-release.yml`. The workflow:

1. Runs on pushes to the `development` branch
2. Executes all cross-platform compatibility tests on Windows, macOS, and Linux
3. Only pushes to the `release` branch if all tests pass
4. Builds and publishes the application from the `release` branch

## Maintaining the Tests

When adding new features to the application, consider adding corresponding tests to ensure cross-platform compatibility. Pay special attention to:

- File paths (use `Path::join` instead of string concatenation)
- File permissions (use platform-specific code when necessary)
- UI elements that might render differently across platforms
- Platform-specific APIs and features

## Test Dependencies

The tests use the `rand` crate for generating random values. This dependency is defined in the `[dev-dependencies]` section of the `Cargo.toml` file.