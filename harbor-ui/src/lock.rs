use fd_lock::RwLock;
use harbor_client::data_dir;
use std::fs::File;
use std::process::Command;

#[derive(Debug)]
pub struct AppLock {
    // Empty struct - we just use it as a token to show we have the lock
}

impl AppLock {
    pub fn acquire() -> Result<Self, String> {
        let data_dir = data_dir(None);
        if !data_dir.exists() {
            std::fs::create_dir_all(&data_dir)
                .map_err(|e| format!("Failed to create data directory: {e}"))?;
        }

        let lock_file_path = data_dir.join("harbor.lock");

        let file = File::options()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&lock_file_path)
            .map_err(|e| format!("Failed to create/open lock file: {e}"))?;

        let mut lock = RwLock::new(file);

        // Try to acquire a write lock - this will fail if another instance has the lock
        let guard = lock
            .try_write()
            .map_err(|_| "Another instance of Harbor is already running".to_string())?;

        // Intentionally leak both the guard and the lock
        std::mem::forget(guard);
        std::mem::forget(lock);

        Ok(AppLock {})
    }
}

/// Restarts the app
/// The lock file will be cleaned up and the app will be restarted with the same arguments.
pub fn restart_app() {
    // Clean up the lock file
    let lock_file_path = data_dir(None).join("harbor.lock");
    let _ = std::fs::remove_file(lock_file_path);

    let args: Vec<String> = std::env::args().collect();
    let executable = &args[0];

    if let Err(e) = Command::new(executable).args(&args[1..]).spawn() {
        eprintln!("Failed to relaunch: {e}");
        std::process::exit(1);
    }

    std::process::exit(0);
}
