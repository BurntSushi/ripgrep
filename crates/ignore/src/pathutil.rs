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
    if let Some(name) = file_name(path) {
        name.as_encoded_bytes().starts_with(b".")
    } else {
        false
    }
}

/// Strip `prefix` from the `path` and return the remainder.
///
/// If `path` doesn't have a prefix `prefix`, then return `None`.
pub(crate) fn strip_prefix<'a, P: AsRef<Path> + ?Sized>(
    prefix: &'a P,
    path: &'a Path,
) -> Option<&'a Path> {
    #[cfg(unix)]
    fn imp<'a>(prefix: &'a Path, path: &'a Path) -> Option<&'a Path> {
        use std::os::unix::ffi::OsStrExt;

        let prefix = prefix.as_os_str().as_bytes();
        let path = path.as_os_str().as_bytes();
        if prefix.len() > path.len() || prefix != &path[0..prefix.len()] {
            None
        } else {
            Some(&Path::new(OsStr::from_bytes(&path[prefix.len()..])))
        }
    }

    #[cfg(not(unix))]
    fn imp<'a>(prefix: &'a Path, path: &'a Path) -> Option<&'a Path> {
        path.strip_prefix(prefix).ok()
    }

    imp(prefix.as_ref(), path)
}

/// Returns true if this file path is just a file name. i.e., Its parent is
/// the empty string.
pub(crate) fn is_file_name<P: AsRef<Path>>(path: P) -> bool {
    #[cfg(unix)]
    {
        memchr::memchr(b'/', path.as_ref().as_os_str().as_encoded_bytes())
            .is_none()
    }
    #[cfg(not(unix))]
    {
        path.as_ref()
            .parent()
            .map(|p| p.as_os_str().is_empty())
            .unwrap_or(false)
    }
}

/// The final component of the path, if it is a normal file.
///
/// If the path terminates in `.`, `..`, or consists solely of a root of
/// prefix, this will return `None`.
pub(crate) fn file_name<'a, P: AsRef<Path> + ?Sized>(
    path: &'a P,
) -> Option<&'a OsStr> {
    #[cfg(unix)]
    fn imp(path: &Path) -> Option<&OsStr> {
        use std::os::unix::ffi::OsStrExt;

        use memchr::memrchr;

        let path = path.as_os_str().as_bytes();
        if path.is_empty() {
            return None;
        }
        let last_slash = memrchr(b'/', path).map(|i| i + 1).unwrap_or(0);
        let name = &path[last_slash..];
        // Only a final component that is exactly `.` or `..` has no file
        // name. A real name that merely ends in a `.`, like `.foo.`, does.
        if name == b"." || name == b".." {
            return None;
        }
        Some(OsStr::from_bytes(name))
    }

    #[cfg(not(unix))]
    fn imp(path: &Path) -> Option<&OsStr> {
        path.file_name()
    }

    imp(path.as_ref())
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;
    use std::path::Path;

    use super::{file_name, is_hidden_path};

    #[test]
    fn file_name_basic() {
        assert_eq!(file_name("a"), Some(OsStr::new("a")));
        assert_eq!(file_name("a/b"), Some(OsStr::new("b")));
        assert_eq!(file_name("/a/b.txt"), Some(OsStr::new("b.txt")));
    }

    // On Windows, `file_name` defers to `std`, which normalizes a trailing
    // `.` component away, so these only apply to the Unix implementation.
    #[cfg(unix)]
    #[test]
    fn file_name_dot_components() {
        assert_eq!(file_name(""), None);
        assert_eq!(file_name("."), None);
        assert_eq!(file_name(".."), None);
        assert_eq!(file_name("a/."), None);
        assert_eq!(file_name("a/.."), None);
        assert_eq!(file_name("/a/b/.."), None);
    }

    #[test]
    fn file_name_trailing_dot() {
        assert_eq!(file_name("a."), Some(OsStr::new("a.")));
        assert_eq!(file_name("a/.b."), Some(OsStr::new(".b.")));
        assert_eq!(file_name("a/..."), Some(OsStr::new("...")));
        assert_eq!(file_name("a/..b.."), Some(OsStr::new("..b..")));
    }

    #[test]
    fn hidden_trailing_dot() {
        assert!(is_hidden_path(Path::new(".foo")));
        assert!(is_hidden_path(Path::new("a/.foo.")));
        assert!(is_hidden_path(Path::new("a/.foo..")));
        assert!(is_hidden_path(Path::new("a/...")));
        assert!(!is_hidden_path(Path::new("a/foo.")));
        assert!(!is_hidden_path(Path::new("a/.")));
        assert!(!is_hidden_path(Path::new("a/..")));
    }
}
