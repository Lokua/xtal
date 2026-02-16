use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

pub struct ShaderWatch {
    changed: Arc<AtomicBool>,
    _watcher: RecommendedWatcher,
}

impl ShaderWatch {
    pub fn start(path: PathBuf) -> Result<Self, notify::Error> {
        let changed = Arc::new(AtomicBool::new(false));
        let changed_flag = changed.clone();
        let shader_path = path.clone();

        let mut watcher = notify::recommended_watcher(move |result| {
            let Ok(event) = result else {
                return;
            };

            if shader_changed(&event, &shader_path) {
                changed_flag.store(true, Ordering::SeqCst);
            }
        })?;

        watcher.watch(&path, RecursiveMode::NonRecursive)?;

        Ok(Self {
            changed,
            _watcher: watcher,
        })
    }

    pub fn take_changed(&self) -> bool {
        self.changed.swap(false, Ordering::SeqCst)
    }
}

fn shader_changed(event: &Event, shader_path: &PathBuf) -> bool {
    if !matches!(
        event.kind,
        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
    ) {
        return false;
    }

    if event.paths.is_empty() {
        return true;
    }

    event
        .paths
        .iter()
        .any(|path| path_matches_target(path, shader_path))
}

fn path_matches_target(
    path: &std::path::Path,
    target: &std::path::Path,
) -> bool {
    if path == target {
        return true;
    }

    if path.file_name() == target.file_name() {
        return true;
    }

    let path_canon = path.canonicalize().ok();
    let target_canon = target.canonicalize().ok();

    match (path_canon, target_canon) {
        (Some(path_canon), Some(target_canon)) => path_canon == target_canon,
        _ => false,
    }
}
