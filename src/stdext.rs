use core::fmt::Write;
use core::{ffi, fmt};

pub struct LibCStdoutWriter;

impl Write for LibCStdoutWriter {
    fn write_str(&mut self, mut s: &str) -> fmt::Result {
        loop {
            let r = unsafe { libc::write(libc::STDOUT_FILENO, s.as_ptr().cast(), s.len()) };
            if r < 0 {
                return Err(fmt::Error);
            }
            if r == 0 {
                return Ok(());
            }
            s = &s[(r as usize)..];
        }
    }
}

pub fn print(args: fmt::Arguments<'_>) -> fmt::Result {
    write!(LibCStdoutWriter, "{}", args)
}

macro_rules! print {
    ($($tt:tt)*) => {{
       $crate::stdext::print(format_args!($($tt)*)).unwrap();
    }};
}

macro_rules! trace {
    ($($tt:tt)*) => {
        print!("UWUWIND TRACE | uwuwind/{}:{}: {}\n", file!(), line!(), format_args!($($tt)*))
    };
}

pub(crate) use trace;

pub(crate) fn abort() -> ! {
    // SAFETY: We abort.
    unsafe { libc::abort() };
}

fn errno() -> i32 {
    // SAFETY: Surely errno_location would be valid, right?
    unsafe { *libc::__errno_location() }
}

pub(crate) fn with_last_os_error_str<R>(f: impl FnOnce(&str) -> R) -> R {
    let mut buf: [u8; 512] = [0; 512];

    extern "C" {
        // the libc crate only has the definition for the POSIX version, but a GNU system has the GNU version.
        fn strerror_r(errnum: i32, buf: *mut ffi::c_char, buflen: usize) -> i32;
    }

    // SAFETY: Our buffer length is passed correctly
    let error = unsafe { libc::strerror_r(errno(), buf.as_mut_ptr().cast(), buf.len()) };
    // SAFETY: strerror_r writes the string to buf, even if it didnt write anything, we did zero init it.
    let cstr = if error != 0 {
        ffi::CStr::from_bytes_with_nul(b"<strerror_r returned an error>\n").unwrap()
    } else {
        unsafe { ffi::CStr::from_ptr(buf.as_ptr().cast()) }
    };
    f(cstr
        .to_str()
        .unwrap_or("<error message contained invalid utf8>"))
}
