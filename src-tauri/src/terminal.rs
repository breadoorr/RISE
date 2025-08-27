use std::process::Command;

#[tauri::command]
pub async fn open_system_terminal(dir: String) -> Result<String, String> {
    #[cfg(target_os = "macos")]
    {
        if Command::new("open").arg("-a").arg("Terminal").arg(&dir).spawn().is_ok() {
            return Ok("Opened Terminal.app".to_string());
        }
        let osa = format!(
            "tell application \"iTerm\"\n  create window with default profile\n  tell current session of current window to write text \"cd '{}'\"\nend tell",
            dir.replace("'", "'\\''")
        );
        if Command::new("osascript").arg("-e").arg(osa).spawn().is_ok() {
            return Ok("Opened iTerm".to_string());
        }
        return Err("Failed to open Terminal/iTerm".to_string());
    }

    #[cfg(target_os = "windows")]
    {
        if Command::new("wt").args(&["-w", "0", "nt", "-d", &dir]).spawn().is_ok() {
            return Ok("Opened Windows Terminal".to_string());
        }
        if Command::new("cmd").args(&["/K", &format!("cd /D {}", dir)]).spawn().is_ok() {
            return Ok("Opened cmd.exe".to_string());
        }
        return Err("Failed to open Windows terminal".to_string());
    }

    #[cfg(target_os = "linux")]
    {
        let candidates = [
            ("gnome-terminal", vec!["--working-directory", &dir]),
            ("konsole", vec!["--workdir", &dir]),
            ("xfce4-terminal", vec!["--working-directory", &dir]),
            ("x-terminal-emulator", vec!["--working-directory", &dir]),
            ("xterm", vec!["-e", &format!("cd '{}' && exec bash", dir)]),
        ];

        for (bin, args) in candidates {
            if Command::new(bin).args(args).spawn().is_ok() {
                return Ok(format!("Opened {}", bin));
            }
        }
        Err("No suitable terminal found".to_string())
    }
}
