use rfd::FileDialog;

use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let files = FileDialog::new().pick_folder();

    println!("files: {:?}", files);

    Ok(())
}
