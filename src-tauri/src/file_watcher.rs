use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use lazy_static::lazy_static;
use std::collections::{HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

// Self-write suppression: backend operations can mark paths here
lazy_static! {
    static ref RECENT_WRITES: Mutex<VecDeque<(PathBuf, Instant)>> = Mutex::new(VecDeque::new());
    static ref CONTROL_TX: Mutex<Option<Sender<ControlMsg>>> = Mutex::new(None);
}

#[derive(Debug)]
enum ControlMsg {
    SetPath(PathBuf),
    Shutdown,
}

// Public API: mark a path as written by the app (to filter echo events)
pub fn mark_self_write<P: AsRef<Path>>(path: P) {
    let mut q = RECENT_WRITES.lock().unwrap();
    let now = Instant::now();
    q.push_back((path.as_ref().to_path_buf(), now));
    // Trim to keep last N entries
    while q.len() > 256 {
        q.pop_front();
    }
}

// Public API: set or change the watched project path (starts worker on first use)
pub fn set_watched_path(app: AppHandle, path: String) {
    let mut guard = CONTROL_TX.lock().unwrap();
    if guard.is_none() {
        // Start worker thread lazily
        let (tx, rx) = mpsc::channel::<ControlMsg>();
        *guard = Some(tx.clone());
        thread::spawn(move || watcher_worker(app, rx));
    }
    if let Some(tx) = &*guard {
        let _ = tx.send(ControlMsg::SetPath(PathBuf::from(path)));
    }
}

fn watcher_worker(app: AppHandle, ctrl_rx: Receiver<ControlMsg>) {
    // Event channel from notify
    let (evt_tx, evt_rx) = mpsc::channel::<notify::Result<Event>>();
    let mut watcher: Option<RecommendedWatcher> = None;
    let mut watched_path: Option<PathBuf> = None;

    // Batching state
    let mut pending_dirs: HashSet<PathBuf> = HashSet::new();
    let mut last_emit: Instant = Instant::now() - Duration::from_secs(10);
    let debounce = Duration::from_millis(800);
    let min_interval = Duration::from_secs(2);

    loop {
        // Prefer control messages with a short timeout to also check events
        match ctrl_rx.recv_timeout(Duration::from_millis(150)) {
            Ok(ControlMsg::SetPath(new_path)) => {
                // Initialize watcher if needed
                if watcher.is_none() {
                    match notify::recommended_watcher(evt_tx.clone()) {
                        Ok(w) => watcher = Some(w),
                        Err(e) => {
                            eprintln!("File watcher init error: {:?}", e);
                            continue;
                        }
                    }
                }
                // Unwatch old
                if let (Some(w), Some(old)) = (watcher.as_mut(), watched_path.as_ref()) {
                    if let Err(e) = w.unwatch(old) {
                        eprintln!("File watcher unwatch error: {:?}", e);
                    }
                }
                // Watch new
                if let Some(w) = watcher.as_mut() {
                    if let Err(e) = w.watch(&new_path, RecursiveMode::Recursive) {
                        eprintln!("File watcher path error: {:?}", e);
                    } else {
                        watched_path = Some(new_path.clone());
                        // Reset batching state on path change
                        pending_dirs.clear();
                        last_emit = Instant::now() - min_interval;
                    }
                }
            }
            Ok(ControlMsg::Shutdown) => {
                // Drop watcher and exit
                drop(watcher.take());
                return;
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                // No control msg; proceed to check FS events and possibly emit
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => return,
        }

        // Drain FS events without blocking long
        while let Ok(res) = evt_rx.try_recv() {
            match res {
                Ok(event) => collect_event_paths(&mut pending_dirs, &event),
                Err(e) => eprintln!("File watcher error: {:?}", e),
            }
        }

        // Debounce/coalesce and emit if needed
        let now = Instant::now();
        if !pending_dirs.is_empty()
            && now.duration_since(last_emit) >= debounce
            && now.duration_since(last_emit) >= min_interval
        {
            // Emit a single simple event (string payload) to avoid TS typing changes on frontend
            let _ = app.emit("fs-changed", "");
            pending_dirs.clear();
            last_emit = now;
        }

        // Periodically cleanup recent writes queue
        cleanup_recent_writes();
    }
}

fn cleanup_recent_writes() {
    let mut q = RECENT_WRITES.lock().unwrap();
    let now = Instant::now();
    let keep_for = Duration::from_secs(2);
    while let Some((_, t)) = q.front() {
        if now.duration_since(*t) > keep_for {
            q.pop_front();
        } else {
            break;
        }
    }
}

fn collect_event_paths(pending_dirs: &mut HashSet<PathBuf>, event: &Event) {
    for p in &event.paths {
        // Filter noisy paths
        if should_ignore(p) { continue; }
        if is_self_write(p) { continue; }
        // Add the parent dir (or itself if no parent)
        if let Some(dir) = p.parent() {
            pending_dirs.insert(dir.to_path_buf());
        } else {
            pending_dirs.insert(p.to_path_buf());
        }
    }
}

fn should_ignore(p: &Path) -> bool {
    let s = p.to_string_lossy();
    let lowers = s.to_lowercase();
    // Ignore common heavy/noisy dirs
    if lowers.contains("/node_modules/") || lowers.ends_with("/node_modules") { return true; }
    if lowers.contains("/.git/") || lowers.ends_with("/.git") { return true; }
    if lowers.contains("/target/") || lowers.ends_with("/target") { return true; }

    // Ignore temp/backup files
    if s.ends_with("~") { return true; }
    if lowers.ends_with(".swp") || lowers.ends_with(".tmp") || lowers.ends_with(".temp") { return true; }
    false
}

fn is_self_write(p: &Path) -> bool {
    let q = RECENT_WRITES.lock().unwrap();
    let now = Instant::now();
    let window = Duration::from_secs(1);
    for (path, t) in q.iter() {
        if now.duration_since(*t) <= window && path == p {
            return true;
        }
    }
    false
}