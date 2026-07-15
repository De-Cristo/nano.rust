use std::ffi::{c_char, c_void, CStr, CString};
use std::ptr::NonNull;

use crate::error::{Error, Result};

const ERROR_CAPACITY: usize = 1024;

pub(crate) struct XrootdFile {
    raw: NonNull<c_void>,
}

impl XrootdFile {
    pub(crate) fn open(url: &str) -> Result<Self> {
        let url = CString::new(url).map_err(|_| {
            Error::unsupported("XRootD ROOT source", "URL contains an embedded NUL byte")
        })?;
        let mut error = [0 as c_char; ERROR_CAPACITY];
        let raw = unsafe { nano_rootio_xrootd_open(url.as_ptr(), error.as_mut_ptr(), error.len()) };
        let raw = NonNull::new(raw).ok_or_else(|| xrootd_error("open", &error))?;
        Ok(Self { raw })
    }

    pub(crate) fn read(&mut self, offset: u64, len: u64) -> Result<Vec<u8>> {
        let size = u32::try_from(len).map_err(|_| {
            Error::unsupported(
                "XRootD ROOT source",
                format!("read length {len} exceeds the XRootD single-read limit"),
            )
        })?;
        let mut bytes = vec![
            0;
            usize::try_from(len).map_err(|_| {
                Error::unsupported(
                    "XRootD ROOT source",
                    format!("read length {len} overflows usize"),
                )
            })?
        ];
        let mut bytes_read = 0_u32;
        let mut error = [0 as c_char; ERROR_CAPACITY];
        let status = unsafe {
            nano_rootio_xrootd_read(
                self.raw.as_ptr(),
                offset,
                size,
                bytes.as_mut_ptr().cast(),
                &mut bytes_read,
                error.as_mut_ptr(),
                error.len(),
            )
        };
        if status != 0 {
            return Err(xrootd_error("read", &error));
        }
        if u64::from(bytes_read) != len {
            return Err(Error::unsupported(
                "XRootD ROOT source",
                format!(
                    "short read at offset {offset}: requested {len} bytes, received {bytes_read}"
                ),
            ));
        }
        Ok(bytes)
    }
}

impl Drop for XrootdFile {
    fn drop(&mut self) {
        unsafe { nano_rootio_xrootd_close(self.raw.as_ptr()) };
    }
}

fn xrootd_error(operation: &str, error: &[c_char]) -> Error {
    let detail = unsafe { CStr::from_ptr(error.as_ptr()) }
        .to_string_lossy()
        .into_owned();
    Error::unsupported(
        "XRootD ROOT source",
        if detail.is_empty() {
            format!("XRootD {operation} failed")
        } else {
            format!("XRootD {operation} failed: {detail}")
        },
    )
}

unsafe extern "C" {
    fn nano_rootio_xrootd_open(
        url: *const c_char,
        error: *mut c_char,
        error_capacity: usize,
    ) -> *mut c_void;
    fn nano_rootio_xrootd_read(
        handle: *mut c_void,
        offset: u64,
        size: u32,
        buffer: *mut c_void,
        bytes_read: *mut u32,
        error: *mut c_char,
        error_capacity: usize,
    ) -> i32;
    fn nano_rootio_xrootd_close(handle: *mut c_void);
}
