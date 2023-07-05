use std::fs::File;
use std::path::Path;

use memmap::Mmap;

/// Controls the strategy used for determining when to use memory maps.
///
/// If a searcher is called in circumstances where it is possible to use memory
/// maps, and memory maps are enabled, then it will attempt to do so if it
/// believes it will make the search faster.
///
/// By default, memory maps are disabled.
#[derive(Clone, Debug)]
pub struct MmapChoice(MmapChoiceImpl);

#[derive(Clone, Debug)]
enum MmapChoiceImpl {
    Auto,
    Never,
}

impl Default for MmapChoice {
    fn default() -> MmapChoice {
        MmapChoice(MmapChoiceImpl::Never)
    }
}

impl MmapChoice {
    /// Use memory maps when they are believed to be advantageous.
    ///
    /// The heuristics used to determine whether to use a memory map or not
    /// may depend on many things, including but not limited to, file size
    /// and platform.
    ///
    /// If memory maps are unavailable or cannot be used for a specific input,
    /// then normal OS read calls are used instead.
    ///
    /// # Safety
    ///
    /// This constructor is not safe because there is no obvious way to
    /// encapsulate the safety of file backed memory maps on all platforms
    /// without simultaneously negating some or all of their benefits.
    ///
    /// The specific contract the caller is required to uphold isn't precise,
    /// but it basically amounts to something like, "the caller guarantees that
    /// the underlying file won't be mutated." This, of course, isn't feasible
    /// in many environments. However, command line tools may still decide to
    /// take the risk of, say, a `SIGBUS` occurring while attempting to read a
    /// memory map.
    pub unsafe fn auto() -> MmapChoice {
        MmapChoice(MmapChoiceImpl::Auto)
    }

    /// Never use memory maps, no matter what. This is the default.
    pub fn never() -> MmapChoice {
        MmapChoice(MmapChoiceImpl::Never)
    }

    /// Return a memory map if memory maps are enabled and if creating a
    /// memory from the given file succeeded and if memory maps are believed
    /// to be advantageous for performance.
    ///
    /// If this does attempt to open a memory map and it fails, then `None`
    /// is returned and the corresponding error (along with the file path, if
    /// present) is logged at the debug level.
    pub(crate) fn open(
        &self,
        file: &File,
        path: Option<&Path>,
    ) -> Option<Mmap> {
        if !self.is_enabled() {
            return None;
        }
        if cfg!(target_os = "macos") {
            // I guess memory maps on macOS aren't great. Should re-evaluate.
            return None;
        }
        // SAFETY: This is acceptable because the only way `MmapChoiceImpl` can
        // be `Auto` is if the caller invoked the `auto` constructor, which
        // is itself not safe. Thus, this is a propagation of the caller's
        // assertion that using memory maps is safe.
        match unsafe { Mmap::map(file) } {
            Ok(mmap) => Some(mmap),
            Err(err) => {
                if let Some(path) = path {
                    log::debug!(
                        "{}: failed to open memory map: {}",
                        path.display(),
                        err
                    );
                } else {
                    log::debug!("failed to open memory map: {}", err);
                }
                None
            }
        }
    }

    /// Whether this strategy may employ memory maps or not.
    pub(crate) fn is_enabled(&self) -> bool {
        match self.0 {
            MmapChoiceImpl::Auto => true,
            MmapChoiceImpl::Never => false,
        }
    }
}
