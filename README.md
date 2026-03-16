# <img width="100" height="100" alt="logo" src="https://github.com/user-attachments/assets/3af77fa1-47da-48d5-a9d9-cdb989f63cb1" />&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;RISE - IDE for developers from developers

*RISE is a modern desktop development environment built with Tauri, SvelteKit, and Rust.*

**RISE is designed to be a fast, lightweight development workspace that runs as a native desktop application.**

*The project combines:*
* ***Rust + Tauri for system-level functionality***
* ***SvelteKit for a modern reactive UI***
* ***TypeScript for maintainable frontend logic***

*This hybrid architecture provides:*
* native performance
* small bundle size
* cross-platform support
* modern UI capabilities

## Features

### Desktop Application
* Native desktop app powered by Tauri
* Cross-platform support (Linux, macOS, Windows)
* Lightweight runtime compared to Electron


### Integrated Code Editor
* File editing capabilities
* Syntax highlighting
* Theming support
* Project-based file management

  
### Terminal Integration
* Built-in terminal using xterm
* Interactive shell support
* Fit-to-container terminal display


### File System Monitoring
* File watching support
* Automatic refresh on file changes
* Cross-platform filesystem compatibility
### Theme System
* Theme configuration
* Customizable UI appearance


### Project Management
* Load and manage local projects
* File-based project structure handling

## Architecture

RISE follows a hybrid desktop architecture.

```
Frontend (SvelteKit + TypeScript)
        │
        │  Tauri API
        ▼
Backend (Rust)
        │
        ├── File System Operations
        ├── File Watcher
        ├── Syntax Highlighting
        ├── Project Management
        └── System Commands
```
This design allows:
* high performance
* secure system access
* separation of UI and backend logic


## Tech Stack

### Frontend

* SvelteKit
* TypeScript
* Vite
* SCSS
* xterm.js (terminal emulator)

### Backend

* Rust
* Tauri

### Tooling

* Node.js
* npm
* Cargo

## Project Structure

```
RISE
│
├── src/                     # SvelteKit frontend
│   ├── app.html
│   ├── app.css
│   │
│   ├── lib
│   │   ├── stores          # Svelte stores
│   │   └── utils           # Utility helpers
│   │
│   └── routes
│       ├── +layout.svelte
│       ├── +layout.ts
│       ├── +page.svelte
│       └── editor          # Editor interface
│
├── src-tauri/               # Rust backend
│   ├── src
│   │   ├── main.rs
│   │   ├── lib.rs
│   │   ├── actions.rs
│   │   ├── commands.rs
│   │   ├── file_watcher.rs
│   │   ├── highlight.rs
│   │   ├── project.rs
│   │   └── theme.rs
│   │
│   ├── tauri.conf.json
│   └── Cargo.toml
│
├── static/                  # Static assets
│
├── package.json
├── vite.config.js
├── svelte.config.js
└── tsconfig.json
```

## Installation

### Prerequisites

You must have the following installed:
* Node.js (>=18 recommended)
* npm
* Rust
* Cargo
* Tauri CLI

#### Install Tauri CLI:

```bash
cargo install tauri-cli
```

### Running the Application (Development)

#### Clone the repository:

```bash
git clone https://github.com/breadoorr/RISE.git
```

#### Navigate to the project directory:

```bash
cd RISE
```

#### Install frontend dependencies:

```bash
npm install
```

#### Start the development environment:

```bash
npm run tauri dev
```

### Building the Application

To create a production build:

```bash
npm run tauri build
```

The built application will be generated inside:\
`src-tauri/target/release/`

### Testing

Rust backend tests can be executed using:

```bash
cargo test
```

Test suites include:
* filesystem compatibility tests
* platform-specific tests
* UI compatibility tests

Located in:

`src-tauri/tests/`

## Development Workflow

Recommended workflow:

```
main
 └── development
      └── feature/*
```

Steps:
* Create a feature branch from development
* Implement changes
* Test locally
* Submit a pull request

## Contributing

Contributions are welcome.

To contribute:
* Fork the repository
* Create a feature branch
* Commit your changes
* Submit a Pull Request

## License

*This project is licensed under the MIT License.*

## Future Improvements

Planned features include:
* plugin system
* improved syntax highlighting
* workspace management
* customizable editor settings
* performance improvements

