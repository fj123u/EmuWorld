use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

static TEMP_FILE_ID: AtomicU64 = AtomicU64::new(0);

pub fn write(path: &Path, contents: &[u8]) -> io::Result<()> {
    let parent = path.parent().filter(|parent| !parent.as_os_str().is_empty()).unwrap_or(Path::new("."));
    let file_name = path.file_name().ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "path has no file name"))?;

    for _ in 0..100 {
        let id = TEMP_FILE_ID.fetch_add(1, Ordering::Relaxed);
        let temp_path = parent.join(format!(
            ".{}.{}.{}.tmp",
            file_name.to_string_lossy(),
            std::process::id(),
            id
        ));
        let mut temp_file = match OpenOptions::new().write(true).create_new(true).open(&temp_path) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        };

        let result = (|| {
            temp_file.write_all(contents)?;
            temp_file.sync_all()?;
            drop(temp_file);
            fs::rename(&temp_path, path)
        })();

        if result.is_err() {
            let _ = fs::remove_file(&temp_path);
        }
        return result;
    }

    Err(io::Error::new(io::ErrorKind::AlreadyExists, "could not create a unique temporary file"))
}

#[cfg(test)]
mod tests {
    use super::write;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_path() -> PathBuf {
        let id = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        std::env::temp_dir().join(format!("emuworld-atomic-{}-{}.json", std::process::id(), id))
    }

    #[test]
    fn replaces_existing_file_without_leaving_temporary_files() {
        let path = test_path();
        fs::write(&path, b"old data").unwrap();

        write(&path, b"new data").unwrap();

        assert_eq!(fs::read(&path).unwrap(), b"new data");
        let parent = path.parent().unwrap();
        let file_name = path.file_name().unwrap().to_string_lossy();
        assert!(!fs::read_dir(parent).unwrap().flatten().any(|entry| {
            entry.file_name().to_string_lossy().starts_with(&format!(".{}.", file_name))
        }));
        fs::remove_file(path).unwrap();
    }
}
