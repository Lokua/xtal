use std::{env, path::PathBuf};

use crate::framework::prelude::*;

pub fn generate_session_id() -> String {
    uuid(5)
}

pub fn lattice_config_dir() -> Option<PathBuf> {
    directories_next::BaseDirs::new()
        .map(|base| base.config_dir().join("Lattice"))
}

pub fn lattice_project_root() -> PathBuf {
    let mut path = env::current_exe()
        .expect("Failed to get executable path")
        .parent()
        .expect("Failed to get executable directory")
        .to_path_buf();

    while !path.join("Cargo.toml").exists() {
        if let Some(parent) = path.parent() {
            path = parent.to_path_buf();
        } else {
            panic!("Could not find project root directory");
        }
    }

    path
}
