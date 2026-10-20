//! Native project writes that leave the previous file intact until replacement.
//!
//! The temporary file is created beside the destination, so the final rename
//! stays on the same filesystem. This does not claim crash durability: syncing
//! the file is not the same as syncing the containing directory.

use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(0);

struct PendingFile {
    path: PathBuf,
    armed: bool,
}

impl Drop for PendingFile {
    fn drop(&mut self) {
        if self.armed {
            let _ = fs::remove_file(&self.path);
        }
    }
}

/// Write a complete native file before replacing `path` with one rename.
///
/// Existing non-file targets (including symbolic links) are rejected. An
/// existing file's permissions are retained on the replacement. Errors before
/// rename leave the previous destination untouched and remove the temporary
/// file. Like `std::fs::rename`, atomic replacement depends on the host
/// filesystem; the containing directory is not synced here.
pub fn write_file_atomic(path: &Path, contents: &[u8]) -> io::Result<()> {
    write_file_atomic_with(path, contents, |_| Ok(()))
}

fn write_file_atomic_with(
    path: &Path,
    contents: &[u8],
    before_rename: impl FnOnce(&Path) -> io::Result<()>,
) -> io::Result<()> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let name = path
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Save path has no file name"))?;
    let old_permissions = match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_file() => Some(metadata.permissions()),
        Ok(_) => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Save destination is not a regular file",
            ));
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => return Err(error),
    };

    let (mut file, mut pending) = create_sibling_temp(parent, name)?;
    file.write_all(contents)?;
    if let Some(permissions) = old_permissions {
        file.set_permissions(permissions)?;
    }
    file.sync_all()?;
    drop(file);
    before_rename(&pending.path)?;
    fs::rename(&pending.path, path)?;
    // Another process could reuse the old temporary name after rename.
    pending.armed = false;
    Ok(())
}

fn create_sibling_temp(parent: &Path, name: &std::ffi::OsStr) -> io::Result<(File, PendingFile)> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    for _ in 0..64 {
        let id = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
        let mut filename = std::ffi::OsString::from(".");
        filename.push(name);
        filename.push(format!(
            ".obrush-save-{}-{timestamp}-{id}.tmp",
            std::process::id()
        ));
        let path = parent.join(filename);
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => return Ok((file, PendingFile { path, armed: true })),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "Cannot allocate a unique temporary save file",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_dir() -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "obrush-atomic-file-{}-{stamp}-{}",
            std::process::id(),
            NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&dir).unwrap();
        dir
    }

    fn no_temp_files(dir: &Path) -> bool {
        fs::read_dir(dir).unwrap().all(|entry| {
            !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .contains(".obrush-save-")
        })
    }

    #[test]
    fn creates_and_replaces_exact_bytes_without_leftover_temporary_file() {
        let dir = fixture_dir();
        let path = dir.join("sculpt.obrush");
        write_file_atomic(&path, b"first\0project").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"first\0project");
        write_file_atomic(&path, b"second\nproject").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"second\nproject");
        assert!(no_temp_files(&dir));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn injected_pre_rename_failure_keeps_previous_bytes_and_cleans_temp() {
        let dir = fixture_dir();
        let path = dir.join("sculpt.obrush");
        fs::write(&path, b"known good").unwrap();
        let error = write_file_atomic_with(&path, b"new bytes", |_| {
            Err(io::Error::other("injected failure after sync"))
        })
        .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::Other);
        assert_eq!(fs::read(&path).unwrap(), b"known good");
        assert!(no_temp_files(&dir));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn rejects_directory_target_without_creating_temp() {
        let dir = fixture_dir();
        let target = dir.join("a-directory");
        fs::create_dir(&target).unwrap();
        assert_eq!(
            write_file_atomic(&target, b"new bytes").unwrap_err().kind(),
            io::ErrorKind::InvalidInput
        );
        assert!(target.is_dir());
        assert!(no_temp_files(&dir));
        fs::remove_dir_all(dir).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn replacement_preserves_existing_file_mode() {
        use std::os::unix::fs::PermissionsExt;

        let dir = fixture_dir();
        let path = dir.join("sculpt.obrush");
        fs::write(&path, b"old").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
        write_file_atomic(&path, b"new").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"new");
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o640
        );
        assert!(no_temp_files(&dir));
        fs::remove_dir_all(dir).unwrap();
    }
}
