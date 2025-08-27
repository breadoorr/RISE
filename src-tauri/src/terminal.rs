use std::process::Command;

#[tauri::command]
pub async fn open_system_terminal(dir: String) -> Result<String, String> {
    #[cfg(target_os = "macos")]
    {
        // First try default Terminal.app with working directory
        let term = Command::new("open")
            .arg("-a")
            .arg("Terminal")
            .arg(&dir)
            .output()
            .map_err(|e| format!("Failed to open terminal: {}", e))?;
        if term.status.success() {
            return Ok("Opened Terminal.app".to_string());
        }
        // Fallback: try iTerm via AppleScript
        let osa = Command::new("osascript")
            .arg("-e")
            .arg(format!(
                "tell application \"iTerm\"\n  create window with default profile\n  tell current session of current window to write text \"cd '{}'\"\nend tell",
                dir.replace("'", "'\\''")
            ))
            .output()
            .map_err(|e| format!("Failed to open iTerm: {}", e))?;
        if osa.status.success() {
            return Ok("Opened iTerm".to_string());
        }
        return Err(String::from_utf8(term.stderr).unwrap_or("Failed to open Terminal/iTerm".to_string()));
    }

    #[cfg(target_os = "windows")]
    {
        // Prefer Windows Terminal if available, else fallback to cmd
        let wt = Command::new("cmd")
            .arg("/C")
            .arg(format!("wt -w 0 nt -d \"{}\"", dir))
            .output();
        if let Ok(o) = wt { if o.status.success() { return Ok("Opened Windows Terminal".to_string()); } }

        let output = Command::new("cmd")
            .arg("/C")
            .arg("start")
            .arg("cmd")
            .arg("/K")
            .arg(format!("cd /D {}", dir))
            .output()
            .map_err(|e| format!("Failed to open terminal: {}", e))?;
        if output.status.success() {
            Ok("Opened cmd.exe".to_string())
        } else {
            Err(String::from_utf8(output.stderr).unwrap_or("Failed to open cmd.exe".to_string()))
        }
    }

    #[cfg(target_os = "linux")]
    {
        // Try common terminals in order
        let candidates = vec![
            ("gnome-terminal", vec!["--working-directory", &dir]),
            ("konsole", vec!["--workdir", &dir]),
            ("xfce4-terminal", vec!["--working-directory", &dir]),
            ("x-terminal-emulator", vec!["-e", &format!("bash -lc 'cd \"{}\"; exec bash'", dir)]),
            ("xterm", vec!["-e", &format!("bash -lc 'cd \"{}\"; exec bash'", dir)]),
        ];
        for (bin, args) in candidates {
            let mut cmd = Command::new(bin);
            for a in args { cmd.arg(a); }
            if let Ok(out) = cmd.output() {
                if out.status.success() { return Ok(format!("Opened {}", bin)); }
            }
        }
        Err("No suitable terminal found".to_string())
    }
}
