use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let images_dir = Path::new("images");
    let output_file = Path::new("index.md");

    let mut image_paths: Vec<PathBuf> = WalkDir::new(images_dir)
        .into_iter()
        .filter_map(|entry| {
            entry.ok().and_then(|e| {
                if is_supported_image(e.path()) {
                    Some(e.path().to_owned())
                } else {
                    None
                }
            })
        })
        .collect();

    image_paths.sort_by(|a, b| {
        fs::metadata(b)
            .unwrap()
            .modified()
            .unwrap()
            .cmp(&fs::metadata(a).unwrap().modified().unwrap())
    });

    // Generate markdown content
    let mut markdown_content =
        String::from("Files sorted from most to least recent\n\n");

    for path in image_paths {
        let filename = path.file_name().unwrap().to_string_lossy().into_owned();
        let relative_path = path.strip_prefix(".").unwrap_or(&path);

        // Add to markdown
        markdown_content.push_str(&format!("## {}\n\n", filename));
        markdown_content.push_str(&format!(
            "<img src=\"{}\" alt=\"{}\">\n\n",
            relative_path.display(),
            filename
        ));
    }

    // Write to file
    let mut file = File::create(output_file)?;
    file.write_all(markdown_content.as_bytes())?;

    Ok(())
}

fn is_supported_image(path: &Path) -> bool {
    if let Some(extension) = path.extension() {
        matches!(
            extension.to_string_lossy().to_lowercase().as_str(),
            "jpg" | "jpeg" | "png" | "gif"
        )
    } else {
        false
    }
}
