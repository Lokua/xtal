//! WGSL file watcher used by render graph hot reload.
//!
//! `ShaderWatch` watches a shader's parent directory because many editors save
//! by replacing files atomically. Events are filtered back down to the target
//! path, then content hashes suppress reloads for metadata-only changes.

use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use log::{info, trace, warn};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

/// Change detector for one shader file.
///
/// The watcher callback runs off the render hot path and only flips an atomic
/// flag. The runtime consumes that flag with `take_changed` and decides when to
/// rebuild GPU pipelines.
pub struct ShaderWatch {
    changed: Arc<AtomicBool>,
    _watcher: RecommendedWatcher,
}

impl ShaderWatch {
    /// Starts watching a shader file for meaningful content changes.
    pub fn start(path: PathBuf) -> Result<Self, notify::Error> {
        let changed = Arc::new(AtomicBool::new(false));
        let changed_flag = changed.clone();
        let initial_hash = file_content_hash(&path).ok();
        let last_loaded_hash = Arc::new(Mutex::new(initial_hash));
        let shader_path = path.clone();
        let watch_dir = shader_path
            .parent()
            .map(std::path::Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        info!(
            "watching shader file '{}' via directory '{}'",
            shader_path.display(),
            watch_dir.display()
        );

        let mut watcher = notify::recommended_watcher(move |result| {
            let event: Event = match result {
                Ok(event) => event,
                Err(err) => {
                    warn!(
                        "shader watcher failed for '{}': {}",
                        shader_path.display(),
                        err
                    );
                    return;
                }
            };

            trace!(
                "shader watcher event for '{}': {:?} {:?}",
                shader_path.display(),
                event.kind,
                event.paths
            );

            if shader_changed(&event, &shader_path) {
                info!(
                    "shader fs event matched '{}': {:?}",
                    shader_path.display(),
                    event.kind
                );

                let file_hash = match file_content_hash(&shader_path) {
                    Ok(hash) => hash,
                    Err(err) => {
                        trace!(
                            "shader change event before readable file '{}': {}",
                            shader_path.display(),
                            err
                        );
                        return;
                    }
                };

                if let Ok(mut guard) = last_loaded_hash.lock() {
                    if guard
                        .is_some_and(|existing_hash| existing_hash == file_hash)
                    {
                        info!(
                            "shader content unchanged; skipping reload: {}",
                            shader_path.display()
                        );
                        return;
                    }
                    *guard = Some(file_hash);
                }

                changed_flag.store(true, Ordering::SeqCst);
                info!("detected shader change: {}", shader_path.display());
            }
        })?;

        watcher.watch(&watch_dir, RecursiveMode::NonRecursive)?;

        Ok(Self {
            changed,
            _watcher: watcher,
        })
    }

    /// Returns whether the shader changed since the previous call.
    pub fn take_changed(&self) -> bool {
        self.changed.swap(false, Ordering::SeqCst)
    }
}

/// Hashes shader contents so equivalent save events do not trigger reloads.
fn file_content_hash(path: &Path) -> Result<u64, std::io::Error> {
    let bytes = fs::read(path)?;
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    Ok(hasher.finish())
}

/// Returns true when a filesystem event may represent the target shader.
fn shader_changed(event: &Event, shader_path: &Path) -> bool {
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

/// Matches exact, filename-only, or canonicalized paths for editor saves.
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
