//! Implementation of a named-pipe `Connection` on Windows.

#![cfg(windows)]

use std::ptr;
use std::io;
use crate::{Result, Error, Connection};

/// WINAPI bindings.
mod winapi {
    use std::ptr;

    pub type VOID    = std::ffi::c_void;
    pub type LPVOID  = *mut VOID;
    pub type LPCVOID = *const VOID;
    pub type INT     = i32;
    pub type BOOL    = INT;
    pub type UINT    = u32;
    pub type DWORD   = u32;
    pub type LPDWORD = *mut DWORD;
    pub type CHAR    = i8;
    pub type WCHAR   = i16;
    pub type LPCSTR  = *const CHAR;
    pub type LPWSTR  = *mut WCHAR;
    pub type LPCWSTR = *const WCHAR;
    pub type HANDLE  = LPVOID;

    pub const GENERIC_READ        : DWORD  = 0x80000000;
    pub const GENERIC_WRITE       : DWORD  = 0x40000000;
    pub const OPEN_EXISTING       : DWORD  = 3;
    pub const INVALID_HANDLE_VALUE: HANDLE = -1isize as HANDLE;

    #[link(name = "kernel32")]
    extern "system" {
        fn MultiByteToWideChar(
            CodePage      : UINT  ,
            dwFlags       : DWORD ,
            lpMultiByteStr: LPCSTR,
            cbMultiByte   : INT   ,
            lpWideCharStr : LPWSTR,
            cchWideChar   : INT   ,
        ) -> INT;

        pub fn CreateFileW(
            lpFileName           : LPCWSTR,
            dwDesiredAccess      : DWORD  ,
            dwShareMode          : DWORD  ,
            lpSecurityAttributes : LPVOID ,
            dwCreationDisposition: DWORD  ,
            dwFlagsAndAttributes : DWORD  ,
            hTemplateFile        : HANDLE ,
        ) -> HANDLE;

        pub fn CloseHandle(
            hObject: HANDLE,
        ) -> BOOL;

        pub fn ReadFile(
            hFile               : HANDLE ,
            lpBuffer            : LPVOID ,
            nNumberOfBytesToRead: DWORD  ,
            lpNumberOfBytesRead : LPDWORD,
            lpOverlapped        : LPVOID ,
        ) -> BOOL;

        pub fn WriteFile(
            hFile                 : HANDLE ,
            lpBuffer              : LPCVOID,
            nNumberOfBytesToWrite : DWORD  ,
            lpNumberOfBytesWritten: LPDWORD,
            lpOverlapped          : LPVOID ,
        ) -> BOOL;

        pub fn FlushFileBuffers(
            hFile: HANDLE,
        ) -> BOOL;

        pub fn PeekNamedPipe(
            hNamedPipe            : HANDLE ,
            lpBuffer              : LPVOID ,
            nBufferSize           : DWORD  ,
            lpBytesRead           : LPDWORD,
            lpTotalBytesAvail     : LPDWORD,
            lpBytesLeftThisMessage: LPDWORD,
        ) -> BOOL;
    }

    /// Helper to convert a UTF-8 string to UTF-16.
    pub fn utf8_to_utf16(s: &str) -> Box<[WCHAR]> {
        const CP_UTF8: UINT = 65001;
        // Null terminate
        let mut s = s.to_string();
        s.push('\0');
        // Actual conversion
        let len = unsafe{ MultiByteToWideChar(CP_UTF8, 0, s.as_ptr().cast(), -1, ptr::null_mut(), 0) };
        let mut res = Vec::with_capacity(len as usize);
        unsafe {
            MultiByteToWideChar(CP_UTF8, 0, s.as_ptr().cast(), -1, res.as_mut_ptr(), len);
            res.set_len(len as usize);
        }
        res.into_boxed_slice()
    }
}

pub struct NamedPipe {
    handle: winapi::HANDLE,
}

impl NamedPipe {
    fn open(path: &str) -> io::Result<Self> {
        let path = winapi::utf8_to_utf16(path);
        let handle = unsafe { winapi::CreateFileW(
            path.as_ptr(),
            winapi::GENERIC_READ | winapi::GENERIC_WRITE,
            0,
            ptr::null_mut(),
            winapi::OPEN_EXISTING,
            0,
            ptr::null_mut()) };
        if handle == winapi::INVALID_HANDLE_VALUE {
            Err(io::Error::new(
                io::ErrorKind::NotFound, "The pipe could not be opened!"))
        }
        else {
            Ok(Self{ handle })
        }
    }
}

impl io::Read for NamedPipe {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut nread = 0;
        if unsafe { winapi::ReadFile(
            self.handle,
            buf.as_mut_ptr().cast(),
            buf.len() as winapi::DWORD,
            &mut nread,
            ptr::null_mut()) } == 0 {

            Err(io::Error::new(io::ErrorKind::Other, "Could not read!"))
        }
        else {
            Ok(nread as usize)
        }
    }
}

impl io::Write for NamedPipe {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut nwritten = 0;
        if unsafe { winapi::WriteFile(
            self.handle,
            buf.as_ptr().cast(),
            buf.len() as winapi::DWORD,
            &mut nwritten,
            ptr::null_mut()) } == 0 {

            Err(io::Error::new(io::ErrorKind::Other, "Could not write!"))
        }
        else {
            Ok(nwritten as usize)
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        if unsafe{ winapi::FlushFileBuffers(self.handle) } == 0 {
            Err(io::Error::new(io::ErrorKind::Other, "Could not flush!"))
        }
        else {
            Ok(())
        }
    }
}

impl Connection for NamedPipe {
    fn connect(index: usize) -> Result<Self> {
        let address = format!(r#"\\.\pipe\discord-ipc-{}"#, index);
        Ok(Self::open(&address)?)
    }

    fn can_read(&mut self) -> Result<bool> {
        let mut navail = 0;
        if unsafe { winapi::PeekNamedPipe(
            self.handle,
            ptr::null_mut(),
            0,
            ptr::null_mut(),
            &mut navail,
            ptr::null_mut()) } == 0 {

            Err(Error::IoError(
                io::Error::new(io::ErrorKind::Other, "Could not peek!")))
        }
        else {
            Ok(navail != 0)
        }
    }
}

impl Drop for NamedPipe {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe {
                winapi::CloseHandle(self.handle);
            }
        }
    }
}
