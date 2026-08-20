use std::path::Path;

pub(super) trait DiskSpaceProbe: Send + Sync {
    fn available_bytes(&self, path: &Path) -> std::io::Result<Option<u64>>;
}

pub(super) struct SystemDiskSpaceProbe;

impl DiskSpaceProbe for SystemDiskSpaceProbe {
    fn available_bytes(&self, path: &Path) -> std::io::Result<Option<u64>> {
        available_disk_space(path)
    }
}

#[cfg(unix)]
fn available_disk_space(path: &Path) -> std::io::Result<Option<u64>> {
    use std::ffi::CString;
    use std::mem::MaybeUninit;
    use std::os::unix::ffi::OsStrExt;

    let c_path = CString::new(path.as_os_str().as_bytes()).map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "model install path contains an embedded NUL byte",
        )
    })?;
    let mut stats = MaybeUninit::<libc::statvfs>::zeroed();
    // SAFETY: `c_path` is NUL-terminated and lives across this call, while
    // `stats` points to writable storage large enough for one `statvfs` value.
    let result = unsafe { libc::statvfs(c_path.as_ptr(), stats.as_mut_ptr()) };
    if result != 0 {
        return Err(std::io::Error::last_os_error());
    }
    // SAFETY: a zero return from statvfs initializes the output structure.
    let stats = unsafe { stats.assume_init() };
    let available = u128::from(stats.f_bavail).saturating_mul(u128::from(stats.f_frsize));
    let available = u64::try_from(available).unwrap_or(u64::MAX);
    Ok(Some(available))
}

#[cfg(not(unix))]
fn available_disk_space(_path: &Path) -> std::io::Result<Option<u64>> {
    Ok(None)
}
