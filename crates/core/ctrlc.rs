/// Handle Ctrl-C / SIGINT so that `[ANSI COLOR]Hello, Wor<Ctrl-C here>`
/// always writes `[/ANSI]` before exiting.
///
/// Overview
/// - When starting threads which write to the terminal, each one reports
///   its thread id [^1]. The first thread installs a handler.
/// - If this handler is being run on ^C [^2], stop all reported threads [^3],
///   then write `[/ANSI]` directly to the terminal, bypassing all abstractions
///   and possibly held locks (so be async-signal-safe). Then exit.
/// - If no ^C was sent, uninstall the handler after joining the threads.
///
/// 1: Not using the scoped join handles because these (unlike non-scoped) can
///    not be converted into the required platform specific ones.
/// 2: On Unix, only the first ^C is handled, the second one will directly terminate
///    the program as usual.
/// 3: On Unix, the thread currently handling SIGINT "stops" the others by sending
///    SIGUSR1 to them, which will then stop/pause these in the signal handler.
///    On Windows the handler is started in a new thread, and `SuspendThread()`
///    is called on all other threads, after which the handler also needs to be
///    async-signal-safe.
///
/// ALL unsafe blocks are used to call `libc` or `winapi` functions, or (once) to
/// "memset" a C-struct to zero.
/// There is an unavoidable race condition when stopping a thread, by the time the
/// syscall is made the thread could have vanished already - but both Windows and
/// POSIX support it (a future POSIX standard might change that, see the notes in
/// pthread_kill(3) on Linux).
/// It is also possible but harmless to overlook a thread, see `NO_SUCH_THREAD_YET`.

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
mod ctrlc {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, OnceLock};

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    use unix::*;
    #[cfg(target_os = "windows")]
    use windows::*;

    // A slice is sync when T is: `impl<T> Sync for [T] where T: Sync`
    #[derive(Debug)]
    struct VecOfAtomics {
        values: &'static [AtomicUsize],
    }

    impl VecOfAtomics {
        fn new(values: &'static [AtomicUsize]) -> Self {
            VecOfAtomics { values }
        }
    }

    // This is an acceptable lie: There is no reserved value for "no thread", but when
    // exiting via ^C it is ok to miss a thread in extremely rare cases.
    const NO_SUCH_THREAD_YET: ThreadType = usize::MAX as _;

    // Used as [(Number of Threads), (ThreadId1), (ThreadId2), (ThreadId3), ...], and
    // `THREAD_COUNTER_IDX` points to the (Number of Threads) field.
    static THREAD_INFO: OnceLock<Arc<VecOfAtomics>> = OnceLock::new();
    const THREAD_COUNTER_IDX: usize = 0;

    // On Windows two different thread handles can point to the same thread, so no `Eq`
    #[cfg_attr(not(target_os = "windows"), derive(PartialEq, Eq))]
    #[derive(Debug)]
    struct ThreadId(ThreadType);
    impl ThreadId {
        fn valid(&self) -> bool {
            self.raw() != NO_SUCH_THREAD_YET
        }
        fn raw(&self) -> ThreadType {
            self.0
        }
    }

    #[test]
    fn thread_type_and_usize_eq() {
        // better: static_assert
        assert_eq!(
            std::mem::size_of::<AtomicUsize>(),
            std::mem::size_of::<ThreadType>()
        );
    }

    /// Setup global data structure and return `begin()` function to be called in
    /// each thread, and `post_join()` function to be called after joining all
    /// threads.
    pub(crate) fn guard_init(enable: bool, threads: usize) -> (fn(), fn()) {
        if !enable {
            return super::guard_init_disabled();
        }
        let mut values = Vec::with_capacity(threads + 1);
        values.push(AtomicUsize::new(0));
        for _ in 0..threads {
            values.push(AtomicUsize::new(NO_SUCH_THREAD_YET as usize));
        }
        debug_assert_eq!(values.len(), threads + 1);
        let values_ref: &'static mut [_] = values.leak();

        THREAD_INFO.get_or_init(|| Arc::new(VecOfAtomics::new(values_ref)));

        (guard_begin, post_join)
    }

    pub(self) fn get_thread_info() -> &'static Arc<VecOfAtomics> {
        THREAD_INFO.get().expect("THREAD_INFO initialized")
    }

    fn guard_begin() {
        let threads = get_thread_info();
        let num_active_threads =
            threads.values[THREAD_COUNTER_IDX].fetch_add(1, Ordering::SeqCst);
        let my_idx = num_active_threads + 1;

        if my_idx >= threads.values.len() {
            debug_assert!(false, "threads miscounted");
            return;
        }

        let this_thread = thread_self();
        threads.values[my_idx]
            .store(this_thread.raw() as usize, Ordering::SeqCst);

        if num_active_threads == 0 {
            enable_actions();
        }
    }

    fn post_join() {
        reset_actions();

        // Windows: The thread handles obtained via `DuplicateHandle()` should be closed, but
        // a) this is just before the program exists, and
        // b) not really: https://devblogs.microsoft.com/oldnewthing/20161215-00/?p=94945
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    pub(crate) mod unix {
        use super::*;
        use libc;
        use std::io;

        pub type ThreadType = libc::pthread_t;

        // Add a newline so "..abababab^C/prompt/here$" is avoided, the '^C' is usually
        // added by the shell itself (and might still be ANSI mis-colored).
        const ANSI_RESET: &str = "\u{1B}[00m\n";

        pub(super) fn thread_self() -> ThreadId {
            // SAFETY: thread_self(3): "This function always succeeds, returning the calling thread's ID."
            ThreadId(unsafe { libc::pthread_self() })
        }

        extern "C" fn on_sigint_or_usr1(
            sig: libc::c_int,
            _info: *mut libc::siginfo_t,
            _data: *mut libc::c_void,
        ) {
            if sig == libc::SIGUSR1 {
                // In case of comically bad luck with the scheduler: Don't loop, but wait, then exit.
                std::thread::sleep(std::time::Duration::from_millis(77));
            } else if sig == libc::SIGINT {
                let threads = get_thread_info();
                let this_thread = thread_self();

                for thread_id in &threads.values[THREAD_COUNTER_IDX + 1..] {
                    let thread_id = ThreadId(
                        thread_id.load(Ordering::SeqCst) as ThreadType
                    );
                    if thread_id.valid() && this_thread != thread_id {
                        // SAFETY: A signal handler (this one) for SIGUSR was installed.
                        // An invalid thread id is just an error (and ignored) according to
                        // pthread_kill(3), e.g. on macOS. However, the Linux man-page references
                        // POSIX.1-2008, noting a possible *future* change:
                        /*
                        But note also that POSIX
                        says that an attempt to use a thread ID whose lifetime has ended produces
                        undefined  behavior, and an attempt to use an invalid thread ID in a call
                        to pthread_kill() can, for example, cause a segmentation fault. */
                        if unsafe {
                            libc::pthread_kill(thread_id.raw(), libc::SIGUSR1)
                        } != 0
                        {
                            // thread does not exist anymore, ignore
                        }
                    }
                }

                let _ = unsafe {
                    // SAFETY: correctness of `buf` and `count` is ensured by Rust. A bad
                    // file descriptor would report an error (ignored). Short writes are also
                    // ignored.
                    libc::write(
                        libc::STDOUT_FILENO,
                        ANSI_RESET.as_ptr() as *const _,
                        ANSI_RESET.len(),
                    )
                };
            } else {
                unreachable!()
            }

            // By convention: 128 + signal number = 130 for SIGTERM
            std::process::exit(130);
        }

        #[derive(Debug, PartialEq)]
        enum Action {
            InstallOneshot,
            InstallPermanent,
            Reset,
        }

        fn sigaction(what: Action, sig: libc::c_int) -> io::Result<()> {
            // SAFETY: zeroes the C struct
            let mut action: libc::sigaction = unsafe { std::mem::zeroed() };

            match what {
                Action::InstallOneshot => {
                    action.sa_sigaction = on_sigint_or_usr1 as _;
                    action.sa_flags = libc::SA_RESETHAND | libc::SA_SIGINFO;
                }
                Action::InstallPermanent => {
                    action.sa_sigaction = on_sigint_or_usr1 as _;
                    action.sa_flags = libc::SA_SIGINFO;
                }
                Action::Reset => {
                    action.sa_sigaction = libc::SIG_DFL;
                }
            }

            let mut retries = 3;
            loop {
                // SAFETY: signal handler is installed or reset to default behavior, assuming
                // the `action` struct is valid (see above).
                if unsafe {
                    libc::sigaction(sig, &action, std::ptr::null_mut()) != 0
                } {
                    match io::Error::last_os_error().raw_os_error() {
                        // should never be zero / None
                        Some(libc::EAGAIN) if retries > 0 => retries -= 1,
                        _ => {
                            break Err(io::Error::last_os_error());
                        }
                    }
                } else {
                    break Ok(());
                }
            }
        }

        pub(super) fn enable_actions() {
            let _ = sigaction(Action::InstallOneshot, libc::SIGINT);
            let _ = sigaction(Action::InstallPermanent, libc::SIGUSR1);
        }

        pub(super) fn reset_actions() {
            let _ = sigaction(Action::Reset, libc::SIGINT);
            let _ = sigaction(Action::Reset, libc::SIGUSR1);
        }
    }

    #[cfg(target_os = "windows")]
    mod windows {
        use super::*;

        use winapi::shared::minwindef::{BOOL, DWORD, FALSE, TRUE};
        use winapi::um::consoleapi::{SetConsoleCtrlHandler, WriteConsoleA};
        use winapi::um::handleapi::DuplicateHandle;
        use winapi::um::processenv::GetStdHandle;
        use winapi::um::processthreadsapi::{
            GetCurrentProcess, GetCurrentThread, SuspendThread,
        };
        use winapi::um::winbase::STD_OUTPUT_HANDLE;
        use winapi::um::wincon::CTRL_C_EVENT;
        use winapi::um::winnt::{DUPLICATE_SAME_ACCESS, HANDLE};

        pub type ThreadType = HANDLE;

        // Add '^C', which is printed when no handler is installed (but not with
        // msys / mingw64 shells). No additional newline is needed.
        const ANSI_RESET: &str = "\u{1B}[00m^C";

        pub(super) fn thread_self() -> ThreadId {
            let mut this_thread: ThreadType = std::ptr::null_mut();

            // SAFETY: GetCurrentThread/Process can not fail. The thread handle on a failed
            // DuplicateHandle() call is not used.
            let success = unsafe {
                let this_process = GetCurrentProcess();
                DuplicateHandle(
                    this_process,
                    GetCurrentThread(),
                    this_process,
                    &mut this_thread,
                    0,
                    FALSE,
                    DUPLICATE_SAME_ACCESS,
                )
            };
            ThreadId(if success == TRUE {
                this_thread
            } else {
                NO_SUCH_THREAD_YET
            })
        }

        extern "system" fn on_ctrlc(event_type: DWORD) -> BOOL {
            if event_type == CTRL_C_EVENT {
                // Observed behavior: When in a Ctrl-C handler (i.e. this), resetting it so the
                // next ^C is not handled by it does not work, this handler has to run to
                // completion. The OneShot / SA_RESETHAND POSIX behavior can not be
                // replicated by calling `reset_actions()` here.

                let threads = get_thread_info();

                for thread_id in &threads.values[THREAD_COUNTER_IDX + 1..] {
                    let thread_id = ThreadId(
                        thread_id.load(Ordering::SeqCst) as ThreadType
                    );
                    if thread_id.valid() {
                        // SAFETY: Not suspending a thread is ok
                        let _ = unsafe { SuspendThread(thread_id.raw()) };
                    }
                }

                // SAFETY: Only a valid handle is used later.
                let stdout_handle = unsafe { GetStdHandle(STD_OUTPUT_HANDLE) };

                if stdout_handle != std::ptr::null_mut() {
                    let mut _bytes_written: DWORD = 0;

                    // Short writes or other errors are ignored.
                    // SAFETY: correctness of `lpBuffer` and `nNumberOfCharsToWrite` is ensured by Rust.
                    let _ = unsafe {
                        WriteConsoleA(
                            stdout_handle,
                            ANSI_RESET.as_ptr() as *const _,
                            ANSI_RESET.len() as DWORD,
                            &mut _bytes_written,
                            std::ptr::null_mut(),
                        )
                    };
                }

                std::process::exit(130);
            } else {
                FALSE
            }
        }

        pub(super) fn enable_actions() {
            unsafe {
                SetConsoleCtrlHandler(Some(on_ctrlc), TRUE);
            }
        }

        pub(super) fn reset_actions() {
            unsafe {
                SetConsoleCtrlHandler(Some(on_ctrlc), FALSE);
            }
        }
    }
}

#[cfg(not(any(
    target_os = "linux",
    target_os = "macos",
    target_os = "windows"
)))]
mod ctrlc {
    pub fn guard_init(_enable: bool, _threads: usize) -> (fn(), fn()) {
        super::guard_init_disabled()
    }
}

fn guard_init_disabled() -> (fn(), fn()) {
    fn nop() {}
    (nop, nop)
}

pub(crate) use ctrlc::guard_init;
