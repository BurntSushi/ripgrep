use std::{ffi::OsStr, path::Path};

use crate::walk::DirEntry;

/// Returns true if and only if this path is considered to be hidden.
///
/// # Platform behavior
///
/// ## Windows
///
/// This returns true if one of the following is true:
///
/// * The base name of the path starts with a `.`.
/// * The file attributes have the `HIDDEN` property set.
///
/// ## All other platforms
///
/// This only returns true if the base name of the path starts with a `.`.
pub(crate) fn is_hidden_path(dent: &Path) -> bool {
    #[cfg(not(windows))]
    fn imp(path: &Path) -> bool {
        is_hidden_path_only(path)
    }

    #[cfg(windows)]
    fn imp(path: &Path) -> bool {
        use std::os::windows::fs::MetadataExt;
        use winapi_util::file;

        if let Ok(md) = path.metadata() {
            if file::is_hidden(md.file_attributes() as u64) {
                return true;
            }
        }
        is_hidden_path_only(path)
    }

    imp(dent)
}

/// Returns true if and only if this directory entry is considered to be
/// hidden.
///
/// # Platform behavior
///
/// ## Windows
///
/// This returns true if one of the following is true:
///
/// * The base name of the path starts with a `.`.
/// * The file attributes have the `HIDDEN` property set.
///
/// ## All other platforms
///
/// This only returns true if the base name of the path starts with a `.`.
pub(crate) fn is_hidden_entry(dent: &DirEntry) -> bool {
    #[cfg(not(windows))]
    fn imp(dent: &DirEntry) -> bool {
        is_hidden_path_only(dent.path())
    }

    #[cfg(windows)]
    fn imp(dent: &DirEntry) -> bool {
        use std::os::windows::fs::MetadataExt;
        use winapi_util::file;

        // This looks like we're doing an extra stat call, but on Windows, the
        // directory traverser reuses the metadata retrieved from each directory
        // entry and stores it on the DirEntry itself. So this is "free."
        if let Ok(md) = dent.metadata() {
            if file::is_hidden(md.file_attributes() as u64) {
                return true;
            }
        }
        is_hidden_path_only(dent.path())
    }

    imp(dent)
}

/// Returns true if and only if this path is considered to be hidden from only
/// the path itself.
///
/// This has the same behavior on all platforms.
fn is_hidden_path_only(path: &Path) -> bool {
    #[cfg(unix)]
    fn imp(path: &Path) -> bool {
        use std::os::unix::ffi::OsStrExt;

        if let Some(name) = file_name(path) {
            name.as_bytes().get(0) == Some(&b'.')
        } else {
            false
        }
    }

    #[cfg(not(unix))]
    fn imp(path: &Path) -> bool {
        if let Some(name) = file_name(path) {
            name.to_str().map(|s| s.starts_with(".")).unwrap_or(false)
        } else {
            false
        }
    }

    imp(path)
}

/// Strip `prefix` from the `path` and return the remainder.
///
/// If `path` doesn't have a prefix `prefix`, then return `None`.
#[cfg(unix)]
pub(crate) fn strip_prefix<'a, P: AsRef<Path> + ?Sized>(
    prefix: &'a P,
    path: &'a Path,
) -> Option<&'a Path> {
    use std::os::unix::ffi::OsStrExt;

    let prefix = prefix.as_ref().as_os_str().as_bytes();
    let path = path.as_os_str().as_bytes();
    if prefix.len() > path.len() || prefix != &path[0..prefix.len()] {
        None
    } else {
        Some(&Path::new(OsStr::from_bytes(&path[prefix.len()..])))
    }
}

/// Strip `prefix` from the `path` and return the remainder.
///
/// If `path` doesn't have a prefix `prefix`, then return `None`.
#[cfg(not(unix))]
pub(crate) fn strip_prefix<'a, P: AsRef<Path> + ?Sized>(
    prefix: &'a P,
    path: &'a Path,
) -> Option<&'a Path> {
    path.strip_prefix(prefix).ok()
}

/// Returns true if this file path is just a file name. i.e., Its parent is
/// the empty string.
#[cfg(unix)]
pub(crate) fn is_file_name<P: AsRef<Path>>(path: P) -> bool {
    use std::os::unix::ffi::OsStrExt;

    use memchr::memchr;

    let path = path.as_ref().as_os_str().as_bytes();
    memchr(b'/', path).is_none()
}

/// Returns true if this file path is just a file name. i.e., Its parent is
/// the empty string.
#[cfg(not(unix))]
pub(crate) fn is_file_name<P: AsRef<Path>>(path: P) -> bool {
    path.as_ref().parent().map(|p| p.as_os_str().is_empty()).unwrap_or(false)
}

/// The final component of the path, if it is a normal file.
///
/// If the path terminates in ., .., or consists solely of a root of prefix,
/// file_name will return None.
#[cfg(unix)]
pub(crate) fn file_name<'a, P: AsRef<Path> + ?Sized>(
    path: &'a P,
) -> Option<&'a OsStr> {
    use memchr::memrchr;
    use std::os::unix::ffi::OsStrExt;

    let path = path.as_ref().as_os_str().as_bytes();
    if path.is_empty() {
        return None;
    } else if path.len() == 1 && path[0] == b'.' {
        return None;
    } else if path.last() == Some(&b'.') {
        return None;
    } else if path.len() >= 2 && &path[path.len() - 2..] == &b".."[..] {
        return None;
    }
    let last_slash = memrchr(b'/', path).map(|i| i + 1).unwrap_or(0);
    Some(OsStr::from_bytes(&path[last_slash..]))
}

/// The final component of the path, if it is a normal file.
///
/// If the path terminates in ., .., or consists solely of a root of prefix,
/// file_name will return None.
#[cfg(not(unix))]
pub(crate) fn file_name<'a, P: AsRef<Path> + ?Sized>(
    path: &'a P,
) -> Option<&'a OsStr> {
    path.as_ref().file_name()
}
