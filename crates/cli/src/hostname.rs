use std::{ffi::OsString, io};

/// Returns the hostname of the current system.
///
/// It is unusual, although technically possible, for this routine to return
/// an error. It is difficult to list out the error conditions, but one such
/// possibility is platform support.
///
/// # Platform specific behavior
///
/// On Windows, this currently uses the "physical DNS hostname" computer name.
/// This may change in the future.
///
/// On Unix, this returns the result of the `gethostname` function from the
/// `libc` linked into the program.
pub fn hostname() -> io::Result<OsString> {
    #[cfg(windows)]
    {
        use winapi_util::sysinfo::{get_computer_name, ComputerNameKind};
        get_computer_name(ComputerNameKind::PhysicalDnsHostname)
    }
    #[cfg(unix)]
    {
        gethostname()
    }
    #[cfg(not(any(windows, unix)))]
    {
        Err(io::Error::new(
            io::ErrorKind::Other,
            "hostname could not be found on unsupported platform",
        ))
    }
}

#[cfg(unix)]
fn gethostname() -> io::Result<OsString> {
    use std::os::unix::ffi::OsStringExt;

    // SAFETY: There don't appear to be any safety requirements for calling
    // sysconf.
    let limit = unsafe { libc::sysconf(libc::_SC_HOST_NAME_MAX) };
    if limit == -1 {
        // It is in theory possible for sysconf to return -1 for a limit but
        // *not* set errno, in which case, io::Error::last_os_error is
        // indeterminate. But untangling that is super annoying because std
        // doesn't expose any unix-specific APIs for inspecting the errno. (We
        // could do it ourselves, but it just doesn't seem worth doing?)
        return Err(io::Error::last_os_error());
    }
    let Ok(maxlen) = usize::try_from(limit) else {
        let msg = format!("host name max limit ({}) overflowed usize", limit);
        return Err(io::Error::new(io::ErrorKind::Other, msg));
    };
    // maxlen here includes the NUL terminator.
    let mut buf = vec![0; maxlen];
    // SAFETY: The pointer we give is valid as it is derived directly from a
    // Vec. Similarly, `maxlen` is the length of our Vec, and is thus valid
    // to write to.
    let rc = unsafe {
        libc::gethostname(buf.as_mut_ptr().cast::<libc::c_char>(), maxlen)
    };
    if rc == -1 {
        return Err(io::Error::last_os_error());
    }
    // POSIX says that if the hostname is bigger than `maxlen`, then it may
    // write a truncate name back that is not necessarily NUL terminated (wtf,
    // lol). So if we can't find a NUL terminator, then just give up.
    let Some(zeropos) = buf.iter().position(|&b| b == 0) else {
        let msg = "could not find NUL terminator in hostname";
        return Err(io::Error::new(io::ErrorKind::Other, msg));
    };
    buf.truncate(zeropos);
    buf.shrink_to_fit();
    Ok(OsString::from_vec(buf))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn print_hostname() {
        println!("{:?}", hostname().unwrap());
    }
}
