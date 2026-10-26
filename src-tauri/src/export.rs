use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use image::{ImageFormat, RgbaImage};

pub fn png(image: &RgbaImage) -> Result<Vec<u8>, String> {
    let mut bytes = io::Cursor::new(Vec::new());
    image
        .write_to(&mut bytes, ImageFormat::Png)
        .map_err(|e| format!("Could not encode the PNG: {e}"))?;
    Ok(bytes.into_inner())
}

/// create_new is atomic: existing screenshots are never overwritten.
fn write_unique(directory: &Path, stem: &str, bytes: &[u8]) -> io::Result<PathBuf> {
    for suffix in 0..10_000 {
        let path = directory.join(format!("{stem}-{suffix}.png"));
        let mut file = match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => file,
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        };
        if let Err(e) = file.write_all(bytes).and_then(|()| file.sync_all()) {
            drop(file);
            // Remove only the incomplete export created by this invocation.
            if let Err(cleanup) = fs::remove_file(&path) {
                return Err(io::Error::new(
                    e.kind(),
                    format!(
                        "{e}; incomplete export {} could not be removed: {cleanup}",
                        path.display()
                    ),
                ));
            }
            return Err(e);
        }
        return Ok(path);
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "Too many filename collisions",
    ))
}

#[cfg(test)]
pub fn save(directory: &Path, image: &RgbaImage) -> Result<PathBuf, String> {
    let bytes = png(image)?;
    save_bytes(directory, &bytes)
}

pub fn save_bytes(directory: &Path, bytes: &[u8]) -> Result<PathBuf, String> {
    fs::create_dir_all(directory).map_err(|e| format!("Could not create export folder: {e}"))?;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("Could not read the system clock: {e}"))?
        .as_millis();
    write_unique(directory, &format!("Slotshot-{timestamp}"), bytes)
        .map_err(|e| format!("Could not save PNG in {}: {e}", directory.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn png_roundtrips_exact_pixels_and_collisions_preserve_existing_file() {
        let dir = std::env::temp_dir().join(format!(
            "slotshot-export-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&dir).unwrap();
        let image = RgbaImage::from_pixel(3, 2, image::Rgba([10, 20, 30, 255]));
        let bytes = png(&image).unwrap();
        let first = write_unique(&dir, "capture", &bytes).unwrap();
        let second = write_unique(&dir, "capture", &bytes).unwrap();
        assert_ne!(first, second);
        assert_eq!(fs::read(&first).unwrap(), bytes);
        assert_eq!(image::open(&second).unwrap().into_rgba8(), image);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn reports_unwritable_destination_without_creating_an_export() {
        let path = std::env::temp_dir().join(format!("slotshot-file-{}", std::process::id()));
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .unwrap();
        assert!(write_unique(&path, "capture", b"test").is_err());
        assert!(save(&path, &RgbaImage::new(1, 1)).is_err());
        drop(file);
        fs::remove_file(path).unwrap();
    }
}
