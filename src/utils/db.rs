//! Safe SQLite database reading utilities
//! Uses snapshot strategy to avoid SQLITE_BUSY errors

use anyhow::Result;
use std::fs;
use std::path::Path;
use tempfile::NamedTempFile;

/// Create a temporary snapshot of a SQLite database for safe reading.
/// This prevents SQLITE_BUSY errors when the IDE has the database locked.
/// Returns the path to the temporary snapshot file.
pub fn create_db_snapshot(source_path: &Path) -> Result<NamedTempFile> {
    if !source_path.exists() {
        anyhow::bail!("Database file does not exist: {:?}", source_path);
    }

    // Create a temporary file with .db extension
    let temp_file = tempfile::Builder::new()
        .prefix("a2zusage-snapshot-")
        .suffix(".db")
        .tempfile()?;

    // Copy the database file to temp
    fs::copy(source_path, temp_file.path())?;

    // Also copy WAL and SHM files if they exist (for WAL mode databases)
    let source_str = source_path.to_string_lossy();
    let wal_path = format!("{}-wal", source_str);
    let shm_path = format!("{}-shm", source_str);

    let temp_str = temp_file.path().to_string_lossy();

    if Path::new(&wal_path).exists() {
        let _ = fs::copy(&wal_path, format!("{}-wal", temp_str));
    }
    if Path::new(&shm_path).exists() {
        let _ = fs::copy(&shm_path, format!("{}-shm", temp_str));
    }

    Ok(temp_file)
}

/// Execute a function with a database snapshot, ensuring cleanup.
/// The temporary files are automatically cleaned up when the returned TempFile is dropped.
pub fn with_db_snapshot<F, T>(source_path: &Path, f: F) -> Result<T>
where
    F: FnOnce(&Path) -> Result<T>,
{
    let snapshot = create_db_snapshot(source_path)?;
    f(snapshot.path())
    // snapshot is automatically cleaned up when dropped
}
