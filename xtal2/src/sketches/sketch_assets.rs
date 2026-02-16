use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct SketchAssets {
    base_dir: PathBuf,
    stem: String,
}

impl SketchAssets {
    pub fn from_file(caller_file: &str) -> Self {
        let caller_path = resolve_caller_path(caller_file);
        let base_dir = caller_path
            .parent()
            .expect("sketch source file has no parent directory")
            .to_path_buf();

        let stem = caller_path
            .file_stem()
            .and_then(|s| s.to_str())
            .expect("sketch source file has no valid UTF-8 stem")
            .to_string();

        Self { base_dir, stem }
    }

    pub fn with_stem(caller_file: &str, stem: impl Into<String>) -> Self {
        let mut assets = Self::from_file(caller_file);
        assets.stem = stem.into();
        assets
    }

    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    pub fn path(&self, relative: impl AsRef<Path>) -> PathBuf {
        self.base_dir.join(relative.as_ref())
    }

    pub fn wgsl(&self) -> PathBuf {
        self.path(format!("{}.wgsl", self.stem))
    }

    pub fn yaml(&self) -> PathBuf {
        self.path(format!("{}.yaml", self.stem))
    }
}

fn resolve_caller_path(caller_file: &str) -> PathBuf {
    let path = PathBuf::from(caller_file);
    if path.is_absolute() {
        return path;
    }

    let cwd = std::env::current_dir()
        .expect("unable to resolve current directory for sketch assets");

    cwd.join(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_stem_and_default_paths() {
        let assets = SketchAssets::from_file("src/sketches/demo.rs");

        assert!(assets.wgsl().ends_with("src/sketches/demo.wgsl"));
        assert!(assets.yaml().ends_with("src/sketches/demo.yaml"));
    }

    #[test]
    fn supports_custom_stem() {
        let assets = SketchAssets::with_stem(
            "src/sketches/compute.rs",
            "compute_present",
        );

        assert!(assets.wgsl().ends_with("src/sketches/compute_present.wgsl"));
    }
}
