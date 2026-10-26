use std::{
    collections::HashSet,
    fs::{self, File},
    io::{Read, Seek, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use image::RgbaImage;
use serde::{Deserialize, Serialize};

use crate::export;

type Result<T> = std::result::Result<T, String>;
static NEXT: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub token: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    version: u8,
    revision: u64,
    slots: Vec<Option<Entry>>,
}

#[derive(Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Settings {
    version: u8,
    export_directory: Option<PathBuf>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct View {
    pub slots: Vec<Option<Entry>>,
    pub revision: u64,
    pub export_directory: Option<String>,
    pub default_directory: String,
    pub warning: Option<String>,
}

#[derive(Serialize)]
pub struct ExportResult {
    pub saved: Vec<String>,
    pub failed: Vec<String>,
}

pub struct Store {
    root: PathBuf,
    config: PathBuf,
    default_directory: PathBuf,
    manifest: Manifest,
    settings: Settings,
    _lock: File,
}

fn error(e: impl std::fmt::Display) -> String {
    format!("Local storage: {e}")
}

fn directory(path: &Path) -> Result<PathBuf> {
    fs::create_dir_all(path).map_err(error)?;
    if fs::symlink_metadata(path)
        .map_err(error)?
        .file_type()
        .is_symlink()
    {
        return Err("Managed storage cannot be a symbolic link.".into());
    }
    fs::canonicalize(path).map_err(error)
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<Option<T>> {
    match fs::symlink_metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(error(e)),
        Ok(meta) if !meta.is_file() || meta.file_type().is_symlink() => {
            Err("Managed metadata must be a regular file.".into())
        }
        Ok(meta) if meta.len() > 64 * 1024 => Err("Managed metadata is too large.".into()),
        Ok(_) => serde_json::from_slice(&fs::read(path).map_err(error)?)
            .map(Some)
            .map_err(error),
    }
}

/// tempfile uses atomic replacement on both macOS and Windows. A failed persist
/// leaves the previous destination intact; the staging file is removed on drop.
fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut temp = tempfile::Builder::new()
        .prefix(".stage-")
        .tempfile_in(path.parent().ok_or("Missing parent directory.")?)
        .map_err(error)?;
    temp.write_all(bytes).map_err(error)?;
    temp.as_file().sync_all().map_err(error)?;
    // Verify the staged bytes before they can become a committed generation or
    // metadata. Stream comparison bounds memory even for a large selected PNG.
    temp.as_file_mut().rewind().map_err(error)?;
    let mut buffer = [0u8; 64 * 1024];
    for expected in bytes.chunks(buffer.len()) {
        temp.as_file_mut()
            .read_exact(&mut buffer[..expected.len()])
            .map_err(error)?;
        if &buffer[..expected.len()] != expected {
            return Err(
                "Staged file verification failed; the previous assignment is unchanged.".into(),
            );
        }
    }
    temp.persist(path).map_err(error)?;
    Ok(())
}

fn sync_directory(path: &Path) -> Result<()> {
    #[cfg(unix)]
    File::open(path)
        .and_then(|file| file.sync_all())
        .map_err(error)?;
    #[cfg(not(unix))]
    let _ = path; // Windows replacement is provided by tempfile's MoveFileExW.
    Ok(())
}

fn valid_token(token: &str) -> bool {
    !token.is_empty() && token.len() <= 64 && token.bytes().all(|c| c.is_ascii_digit() || c == b'-')
}

impl Store {
    pub fn open(root: PathBuf, config: PathBuf, default_directory: PathBuf) -> Result<Self> {
        let root = directory(&root)?;
        let config = directory(&config)?;
        let lock_path = root.join("store.lock");
        if fs::symlink_metadata(&lock_path)
            .is_ok_and(|m| !m.is_file() || m.file_type().is_symlink())
        {
            return Err("Managed lock must be a regular file.".into());
        }
        let lock = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(lock_path)
            .map_err(error)?;
        lock.try_lock().map_err(|_| {
            "Local slots are open in another Slotshot instance. Close it and retry.".to_string()
        })?;

        let manifest = read_json(&root.join("manifest.json"))?.unwrap_or(Manifest {
            version: 1,
            revision: 0,
            slots: vec![None; 5],
        });
        let settings = read_json(&config.join("settings.json"))?.unwrap_or(Settings {
            version: 1,
            ..Settings::default()
        });
        let store = Self {
            root,
            config,
            default_directory,
            manifest,
            settings,
            _lock: lock,
        };
        if store.manifest.version != 1 || store.settings.version != 1 {
            return Err("Unsupported local storage version. No files were removed.".into());
        }
        if !(1..=10).contains(&store.manifest.slots.len()) {
            return Err("Invalid persisted slot count. No files were removed.".into());
        }
        let mut tokens = HashSet::new();
        for entry in store.manifest.slots.iter().flatten() {
            if !valid_token(&entry.token)
                || !tokens.insert(&entry.token)
                || entry.width == 0
                || entry.height == 0
                || u64::from(entry.width) * u64::from(entry.height) > 64 * 1024 * 1024
            {
                return Err("Invalid persisted slot assignment. No files were removed.".into());
            }
        }
        Ok(store)
    }

    fn path(&self, token: &str, thumbnail: bool) -> PathBuf {
        self.root.join(format!(
            "{}-{token}.png",
            if thumbnail { "thumb" } else { "image" }
        ))
    }

    fn trusted_file(&self, token: &str, thumbnail: bool) -> Result<PathBuf> {
        if !valid_token(token) {
            return Err("Invalid image identifier.".into());
        }
        let path = self.path(token, thumbnail);
        let meta = fs::symlink_metadata(&path).map_err(error)?;
        if meta.file_type().is_symlink()
            || !meta.is_file()
            || fs::canonicalize(&path).map_err(error)?.parent() != Some(self.root.as_path())
        {
            return Err("Screenshot must be a regular file in managed storage.".into());
        }
        Ok(path)
    }

    pub fn view(&self) -> View {
        View {
            slots: self.manifest.slots.clone(),
            revision: self.manifest.revision,
            export_directory: self
                .settings
                .export_directory
                .as_ref()
                .map(|p| p.to_string_lossy().into_owned()),
            default_directory: self.default_directory.to_string_lossy().into_owned(),
            warning: self.manifest.slots.iter().flatten()
                .filter_map(|entry| self.trusted_file(&entry.token, false).err())
                .next().map(|e| format!("A slot source is unavailable: {e}. Restore access or Clear the affected slot.")),
        }
    }

    pub fn reconcile(&self) -> Result<()> {
        let retained: HashSet<_> = self
            .manifest
            .slots
            .iter()
            .flatten()
            .flat_map(|e| [self.path(&e.token, false), self.path(&e.token, true)])
            .collect();
        let mut failures = Vec::new();
        for dir in [&self.root, &self.config] {
            for entry in fs::read_dir(dir).map_err(error)? {
                let entry = entry.map_err(error)?;
                let name = entry.file_name();
                let name = name.to_string_lossy();
                let managed = name
                    .strip_prefix("image-")
                    .or_else(|| name.strip_prefix("thumb-"))
                    .and_then(|n| n.strip_suffix(".png"))
                    .is_some_and(valid_token);
                if (name.starts_with(".stage-") || (dir == &self.root && managed))
                    && !retained.contains(&entry.path())
                    && let Err(e) = fs::remove_file(entry.path())
                {
                    failures.push(e.to_string());
                }
            }
        }
        if failures.is_empty() {
            Ok(())
        } else {
            Err(format!(
                "Changes committed, but obsolete files could not be deleted: {}. Retry cleanup or restart Slotshot.",
                failures.join("; ")
            ))
        }
    }

    fn commit(&mut self, mut next: Manifest) -> Result<()> {
        next.revision = self
            .manifest
            .revision
            .checked_add(1)
            .ok_or("Storage revision overflow.")?;
        atomic_write(
            &self.root.join("manifest.json"),
            &serde_json::to_vec(&next).map_err(error)?,
        )?;
        self.manifest = next;
        sync_directory(&self.root)?;
        self.reconcile()
    }

    pub fn validate_target(&self, slot: Option<usize>, revision: u64) -> Result<usize> {
        if revision != self.manifest.revision {
            return Err("Slots changed. Select a destination again.".into());
        }
        match slot {
            Some(id) if id > 0 && id <= self.manifest.slots.len() => Ok(id - 1),
            Some(_) => Err("Invalid slot number.".into()),
            None => Err("Choose a destination slot.".into()),
        }
    }

    #[cfg(test)]
    pub fn assign(
        &mut self,
        slot: Option<usize>,
        revision: u64,
        image: &RgbaImage,
    ) -> Result<usize> {
        self.assign_encoded(slot, revision, image, &export::png(image)?)
    }

    pub fn assign_encoded(
        &mut self,
        slot: Option<usize>,
        revision: u64,
        image: &RgbaImage,
        png: &[u8],
    ) -> Result<usize> {
        let index = self.validate_target(slot, revision)?;
        let dimensions =
            image::ImageReader::with_format(std::io::Cursor::new(png), image::ImageFormat::Png)
                .into_dimensions()
                .map_err(|e| format!("Invalid capture PNG: {e}"))?;
        if image.width() == 0 || image.height() == 0 || dimensions != image.dimensions() {
            return Err("Capture PNG dimensions do not match the acquired region.".into());
        }
        self.reconcile()?;
        let token = format!(
            "{}-{}-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(error)?
                .as_nanos(),
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        );
        // Unique immutable generations: an interrupted write never damages the old slot.
        atomic_write(&self.path(&token, false), png)?;
        let thumbnail = image::imageops::thumbnail(image, 320, 240);
        atomic_write(&self.path(&token, true), &export::png(&thumbnail)?)?;
        sync_directory(&self.root)?;
        let mut next = self.manifest.clone();
        next.slots[index] = Some(Entry {
            token,
            width: image.width(),
            height: image.height(),
        });
        self.commit(next)?;
        Ok(index + 1)
    }

    pub fn selected(&self, ids: &[usize]) -> Result<Vec<&Entry>> {
        if ids.is_empty() || ids.len() > 10 {
            return Err("Select between 1 and 10 occupied slots.".into());
        }
        let mut seen = HashSet::new();
        ids.iter()
            .map(|id| {
                if !seen.insert(id) {
                    return Err("Duplicate slot number.".into());
                }
                id.checked_sub(1)
                    .and_then(|i| self.manifest.slots.get(i))
                    .and_then(Option::as_ref)
                    .ok_or_else(|| "Selection contains an empty or invalid slot.".into())
            })
            .collect()
    }

    pub fn drag_files(&self, ids: &[usize]) -> Result<Vec<PathBuf>> {
        self.selected(ids)?
            .iter()
            .map(|e| self.trusted_file(&e.token, false))
            .collect()
    }

    pub fn thumbnail(&self, id: usize) -> Result<Vec<u8>> {
        let entry = self.selected(&[id])?[0];
        let path = self.path(&entry.token, true);
        // Missing derivatives can be regenerated; a symlink is an error, never followed.
        if !path.try_exists().map_err(error)? {
            let original = self.trusted_file(&entry.token, false)?;
            let thumbnail = image::open(original)
                .map_err(error)?
                .thumbnail(320, 240)
                .into_rgba8();
            atomic_write(&path, &export::png(&thumbnail)?)?;
        }
        fs::read(self.trusted_file(&entry.token, true)?).map_err(error)
    }

    pub fn clear(&mut self, ids: &[usize], revision: u64) -> Result<()> {
        if revision != self.manifest.revision {
            return Err("Slots changed. Review the selection again.".into());
        }
        self.selected(ids)?;
        let mut next = self.manifest.clone();
        for id in ids {
            next.slots[id - 1] = None;
        }
        self.commit(next)
    }

    pub fn resize(&mut self, count: usize, revision: u64, confirmed: bool) -> Result<()> {
        if !(1..=10).contains(&count) {
            return Err("Choose 1–10 slots.".into());
        }
        if revision != self.manifest.revision {
            return Err("Slots changed. Review the count change again.".into());
        }
        if self.manifest.slots.iter().skip(count).any(Option::is_some) && !confirmed {
            return Err("Confirmation required before deleting occupied slots.".into());
        }
        let mut next = self.manifest.clone();
        next.slots.resize(count, None);
        self.commit(next)
    }

    pub fn configure(&mut self, destination: PathBuf) -> Result<()> {
        let destination = directory(&destination)?;
        if destination.starts_with(&self.root) || destination.starts_with(&self.config) {
            return Err("Choose an export directory outside Slotshot's managed storage.".into());
        }
        let probe = tempfile::Builder::new()
            .prefix(".slotshot-check-")
            .tempfile_in(&destination)
            .map_err(error)?;
        probe.as_file().sync_all().map_err(error)?;
        probe.close().map_err(error)?;
        let settings = Settings {
            version: 1,
            export_directory: Some(destination),
        };
        atomic_write(
            &self.config.join("settings.json"),
            &serde_json::to_vec(&settings).map_err(error)?,
        )?;
        self.settings = settings;
        sync_directory(&self.config)
    }

    pub fn export_directory(&self) -> Result<PathBuf> {
        let path = self
            .settings
            .export_directory
            .as_ref()
            .ok_or("Choose an export folder first.")?;
        let canonical = fs::canonicalize(path).map_err(error)?;
        if canonical.starts_with(&self.root) || canonical.starts_with(&self.config) {
            return Err(
                "Export directory now points into managed storage. Change it in Settings.".into(),
            );
        }
        Ok(canonical)
    }

    pub fn export_selected(&self, ids: &[usize]) -> Result<ExportResult> {
        let entries = self.selected(ids)?;
        let directory = self.export_directory()?;
        let mut result = ExportResult {
            saved: vec![],
            failed: vec![],
        };
        for (id, entry) in ids.iter().zip(entries) {
            let save = self
                .trusted_file(&entry.token, false)
                .and_then(|p| fs::read(p).map_err(error))
                .and_then(|bytes| export::save_bytes(&directory, &bytes));
            match save {
                Ok(path) => result.saved.push(path.to_string_lossy().into_owned()),
                Err(e) => result.failed.push(format!("Slot {id}: {e}")),
            }
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open(temp: &tempfile::TempDir) -> Store {
        Store::open(
            temp.path().join("data"),
            temp.path().join("config"),
            temp.path().join("pictures"),
        )
        .unwrap()
    }
    fn image(color: u8) -> RgbaImage {
        RgbaImage::from_pixel(640, 480, image::Rgba([color, 20, 30, 255]))
    }
    fn files(store: &Store) -> Vec<PathBuf> {
        fs::read_dir(&store.root)
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.extension().is_some_and(|e| e == "png"))
            .collect()
    }

    #[test]
    fn every_occupied_destination_replaces_directly_and_rejects_stale_revision() {
        let temp = tempfile::tempdir().unwrap();
        let mut store = open(&temp);
        for slot in 1..=5 {
            store
                .assign(Some(slot), store.view().revision, &image(slot as u8))
                .unwrap();
        }
        let revision = store.view().revision;
        let old = store.drag_files(&[3]).unwrap().remove(0);
        assert!(store.assign(Some(3), revision - 1, &image(9)).is_err());
        assert!(old.exists());
        store.assign(Some(3), revision, &image(9)).unwrap();
        assert!(!old.exists());
        assert_eq!(store.view().slots.iter().filter(|s| s.is_some()).count(), 5);
    }

    #[test]
    fn direct_capture_export_does_not_change_internal_slots_or_revision() {
        let temp = tempfile::tempdir().unwrap();
        let mut store = open(&temp);
        store.configure(temp.path().join("exports")).unwrap();
        store.assign(Some(1), 0, &image(1)).unwrap();
        let before = fs::read(store.root.join("manifest.json")).unwrap();
        let sources = store.drag_files(&[1]).unwrap();
        let exported = crate::export::save(&store.export_directory().unwrap(), &image(2)).unwrap();
        assert!(exported.is_file());
        assert_eq!(fs::read(store.root.join("manifest.json")).unwrap(), before);
        assert_eq!(store.drag_files(&[1]).unwrap(), sources);
        assert_eq!(files(&store).len(), 2);
    }

    #[test]
    fn invalid_pending_png_cannot_replace_an_existing_image() {
        let temp = tempfile::tempdir().unwrap();
        let mut store = open(&temp);
        store.assign(Some(1), 0, &image(1)).unwrap();
        let before = fs::read(store.root.join("manifest.json")).unwrap();
        let files_before = files(&store);
        assert!(
            store
                .assign_encoded(Some(1), 1, &image(2), b"not a PNG")
                .is_err()
        );
        let wrong_size = export::png(&RgbaImage::new(1, 1)).unwrap();
        assert!(
            store
                .assign_encoded(Some(1), 1, &image(2), &wrong_size)
                .is_err()
        );
        assert_eq!(fs::read(store.root.join("manifest.json")).unwrap(), before);
        assert_eq!(files(&store), files_before);
        assert_eq!(
            image::open(&store.drag_files(&[1]).unwrap()[0])
                .unwrap()
                .into_rgba8(),
            image(1)
        );
    }

    #[test]
    fn assignment_replacement_restart_and_derivative_regeneration() {
        let temp = tempfile::tempdir().unwrap();
        let mut store = open(&temp);
        assert_eq!(store.assign(Some(1), 0, &image(10)).unwrap(), 1);
        let old = store.drag_files(&[1]).unwrap().remove(0);
        let thumb = image::load_from_memory(&store.thumbnail(1).unwrap()).unwrap();
        assert_eq!((thumb.width(), thumb.height()), (320, 240));
        assert_eq!(store.assign(Some(1), 1, &image(50)).unwrap(), 1);
        assert!(!old.exists());
        assert_eq!(files(&store).len(), 2);
        let token = store.view().slots[0].as_ref().unwrap().token.clone();
        fs::remove_file(store.path(&token, true)).unwrap();
        assert!(!store.thumbnail(1).unwrap().is_empty());
        drop(store);
        let store = open(&temp);
        assert_eq!(store.view().revision, 2);
        assert_eq!(store.view().slots.len(), 5);
        assert_eq!(
            image::open(&store.drag_files(&[1]).unwrap()[0])
                .unwrap()
                .into_rgba8(),
            image(50)
        );
    }

    #[test]
    fn destination_is_required_and_stale_replacement_preserves_files() {
        let temp = tempfile::tempdir().unwrap();
        let mut store = open(&temp);
        store.resize(1, 0, false).unwrap();
        assert!(store.assign(Some(0), 1, &image(1)).is_err());
        assert!(store.assign(Some(2), 1, &image(1)).is_err());
        assert_eq!(store.assign(Some(1), 1, &image(1)).unwrap(), 1);
        let source = store.drag_files(&[1]).unwrap();
        assert!(
            store
                .assign(None, 2, &image(2))
                .unwrap_err()
                .contains("Choose a destination")
        );
        assert!(store.assign(Some(1), 1, &image(2)).is_err());
        assert_eq!(store.drag_files(&[1]).unwrap(), source);
        assert_eq!(files(&store).len(), 2);
    }

    #[test]
    fn count_confirmation_cancellation_and_stale_revision_preserve_files() {
        let temp = tempfile::tempdir().unwrap();
        let mut store = open(&temp);
        store.assign(Some(5), 0, &image(1)).unwrap();
        let original = store.drag_files(&[5]).unwrap();
        assert!(
            store
                .resize(3, 1, false)
                .unwrap_err()
                .contains("Confirmation")
        );
        assert_eq!(store.view().slots.len(), 5);
        assert!(original[0].exists());
        assert!(store.resize(3, 0, true).is_err());
        store.resize(3, 1, true).unwrap();
        assert!(!original[0].exists());
        assert!(files(&store).is_empty());
        store.resize(10, 2, false).unwrap();
        assert!(store.view().slots.iter().all(Option::is_none));
        assert!(store.resize(0, 3, true).is_err());
        assert!(store.resize(11, 3, true).is_err());
        drop(store);
        assert_eq!(open(&temp).view().slots.len(), 10);
    }

    #[test]
    fn exports_are_independent_collision_safe_and_not_removed_by_slot_changes() {
        let temp = tempfile::tempdir().unwrap();
        let mut store = open(&temp);
        store.configure(temp.path().join("exports")).unwrap();
        store.assign(Some(1), 0, &image(1)).unwrap();
        store.assign(Some(2), 1, &image(2)).unwrap();
        let result = store.export_selected(&[1, 2]).unwrap();
        assert_eq!(result.saved.len(), 2);
        assert!(result.failed.is_empty());
        let pixels: Vec<_> = result.saved.iter().map(|p| fs::read(p).unwrap()).collect();
        store.assign(Some(1), 2, &image(3)).unwrap();
        store.clear(&[1], 3).unwrap();
        store.resize(1, 4, true).unwrap();
        for (path, expected) in result.saved.iter().zip(pixels) {
            assert_eq!(fs::read(path).unwrap(), expected);
        }
        assert!(store.configure(store.root.clone()).is_err());
        drop(store);
        assert!(
            open(&temp)
                .view()
                .export_directory
                .unwrap()
                .ends_with("exports")
        );
    }

    #[test]
    fn recovery_removes_only_owned_orphans_and_interrupted_writes() {
        let temp = tempfile::tempdir().unwrap();
        let mut store = open(&temp);
        store.assign(Some(1), 0, &image(1)).unwrap();
        let orphan = store.path("999-1", false);
        let stage = store.root.join(".stage-interrupted");
        let unrelated = store.root.join("keep.txt");
        for path in [&orphan, &stage, &unrelated] {
            fs::write(path, b"test").unwrap();
        }
        drop(store);
        let store = open(&temp);
        store.reconcile().unwrap();
        assert!(!orphan.exists());
        assert!(!stage.exists());
        assert!(unrelated.exists());
        assert_eq!(files(&store).len(), 2);
    }

    #[test]
    fn failed_replacement_keeps_previous_assignment_and_cleanup_failure_is_retryable() {
        let temp = tempfile::tempdir().unwrap();
        let mut store = open(&temp);
        store.assign(Some(1), 0, &image(1)).unwrap();
        let old = store.drag_files(&[1]).unwrap();
        // Force metadata replacement to fail without relying on root/OS permissions.
        fs::remove_file(store.root.join("manifest.json")).unwrap();
        fs::create_dir(store.root.join("manifest.json")).unwrap();
        assert!(store.assign(Some(1), 1, &image(2)).is_err());
        assert_eq!(store.drag_files(&[1]).unwrap(), old);
        assert!(old[0].exists());
        fs::remove_dir(store.root.join("manifest.json")).unwrap();
        atomic_write(
            &store.root.join("manifest.json"),
            &serde_json::to_vec(&store.manifest).unwrap(),
        )
        .unwrap();
        store.reconcile().unwrap();
        assert_eq!(files(&store).len(), 2);
        let blocked = store.path("888", false);
        fs::create_dir(&blocked).unwrap();
        let error = store.clear(&[1], 1).unwrap_err();
        assert!(error.contains("obsolete files could not be deleted"));
        assert!(store.view().slots[0].is_none());
        fs::remove_dir(blocked).unwrap();
        store.reconcile().unwrap();
        assert!(files(&store).is_empty());
    }

    #[test]
    fn drag_validation_rejects_empty_duplicate_out_of_range_and_missing_sources() {
        let temp = tempfile::tempdir().unwrap();
        let mut store = open(&temp);
        store.assign(Some(1), 0, &image(1)).unwrap();
        store.assign(Some(3), 1, &image(3)).unwrap();
        for ids in [
            vec![],
            vec![0],
            vec![11],
            vec![2],
            vec![1, 1],
            vec![1, 2],
            vec![usize::MAX],
        ] {
            assert!(store.drag_files(&ids).is_err(), "{ids:?}");
        }
        let sources = store.drag_files(&[3, 1]).unwrap();
        assert_eq!(sources.len(), 2);
        assert!(sources.iter().all(|p| p.is_absolute() && p.exists()));
        fs::remove_file(&sources[0]).unwrap();
        assert!(store.drag_files(&[1, 3]).is_err());
        let result = store.export_selected(&[1, 3]);
        assert!(result.is_err()); // Initial setup is required.
        store.configure(temp.path().join("exports")).unwrap();
        let partial = store.export_selected(&[1, 3]).unwrap();
        assert_eq!(partial.saved.len(), 1);
        assert_eq!(partial.failed.len(), 1);
    }

    #[test]
    fn rejects_untrusted_metadata_and_parallel_instances_without_cleanup() {
        let temp = tempfile::tempdir().unwrap();
        let store = open(&temp);
        assert!(
            Store::open(
                store.root.clone(),
                store.config.clone(),
                temp.path().join("exports")
            )
            .is_err()
        );
        let mut invalid = store.manifest.clone();
        invalid.slots[0] = Some(Entry {
            token: "../../outside".into(),
            width: 1,
            height: 1,
        });
        atomic_write(
            &store.root.join("manifest.json"),
            &serde_json::to_vec(&invalid).unwrap(),
        )
        .unwrap();
        let orphan = store.path("777", false);
        fs::write(&orphan, b"keep until metadata can be read").unwrap();
        drop(store);
        assert!(
            Store::open(
                temp.path().join("data"),
                temp.path().join("config"),
                temp.path().join("exports")
            )
            .is_err()
        );
        assert!(orphan.exists());
    }

    #[cfg(unix)]
    #[test]
    fn symlinks_are_never_followed_for_drag_export_or_cleanup() {
        use std::os::unix::fs::symlink;
        let temp = tempfile::tempdir().unwrap();
        let mut store = open(&temp);
        store.assign(Some(1), 0, &image(1)).unwrap();
        let path = store.drag_files(&[1]).unwrap().remove(0);
        let outside = temp.path().join("outside.png");
        fs::write(&outside, b"private").unwrap();
        fs::remove_file(&path).unwrap();
        symlink(&outside, &path).unwrap();
        assert!(store.drag_files(&[1]).is_err());
        store.clear(&[1], 1).unwrap();
        assert_eq!(fs::read(outside).unwrap(), b"private");
        assert!(!path.exists());
    }
}
