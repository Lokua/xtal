use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const S3_BASE_URL: &str =
    "https://s3.us-east-1.amazonaws.com/lokua.net.lattice/images";

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

    let mut markdown_content =
        String::from("Files sorted from most to least recent\n\n");

    for path in image_paths {
        let filename = path.file_name().unwrap().to_string_lossy().into_owned();
        let relative_path = path
            .strip_prefix(images_dir)
            .unwrap_or(&path)
            .to_string_lossy()
            .into_owned();

        markdown_content.push_str(&format!("## {}\n\n", filename));
        markdown_content.push_str(&format!(
            "<img src=\"{}/{}\" alt=\"{}\">\n\n",
            S3_BASE_URL, relative_path, filename
        ));
    }

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
