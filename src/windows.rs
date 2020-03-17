//! Implementation of a named-pipe `Connection` on Windows.

#![cfg(target_os = "windows")]

/// WINAPI bindings.
mod winapi {
    // Type aliases

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

    // Constants

    pub const GENERIC_READ        : DWORD  = 0x80000000;
    pub const GENERIC_WRITE       : DWORD  = 0x40000000;
    pub const OPEN_EXISTING       : DWORD  = 3;
    pub const ERROR_FILE_NOT_FOUND: DWORD  = 2;
    pub const ERROR_PIPE_BUSY     : DWORD  = 231;
    pub const INVALID_HANDLE_VALUE: HANDLE = -1isize as HANDLE;

    // Bindings

    #[link(name = "kernel32")]
    extern "system" {
        pub fn MultiByteToWideChar(
            CodePage      : UINT  ,
            dwFlags       : DWORD ,
            lpMultiByteStr: LPCSTR,
            cbMultiByte   : INT   ,
            lpWideCharStr : LPWSTR,
            cchWideChar   : INT   ,
        ) -> INT;

        pub fn GetLastError() -> DWORD;

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

        pub fn WaitNamedPipeW(
            lpNamedPipeName: LPCWSTR,
            nTimeOut       : DWORD  ,
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
}

use std::ptr;
use winapi::*;
use crate::Connection;

/// Helper to convert a UTF-8 string to UTF-16.
fn utf8_to_utf16(s: &str) -> Box<[WCHAR]> {
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

/// Represents a named pipe `Connection` on Windows.
#[derive(Debug)]
pub struct NamedPipe {
    handle: HANDLE,
}

impl NamedPipe {
    /// Creates a new `NamedPipe`.
    pub fn new() -> Self {
        Self{ handle: INVALID_HANDLE_VALUE }
    }
}

impl Connection for NamedPipe {
    fn open(&mut self) -> bool {
        if self.is_open() {
            return true;
        }
        // Try all 10 slots
        let mut index = 0;
        loop {
            let pipe_name = format!(r#"\\.\pipe\discord-ipc-{}"#, index);
            let pipe_name = utf8_to_utf16(&pipe_name);
            let pipe_name = pipe_name.as_ptr();

            self.handle = unsafe { CreateFileW(
                pipe_name, GENERIC_READ | GENERIC_WRITE, 0, ptr::null_mut(), OPEN_EXISTING, 0, ptr::null_mut()) };
            if self.handle != INVALID_HANDLE_VALUE {
                return true;
            }

            let last_error = unsafe{ GetLastError() };
            if last_error == ERROR_FILE_NOT_FOUND {
                // Can't do anything
                if index < 9 {
                    index += 1;
                    continue;
                }
            }
            else if last_error == ERROR_PIPE_BUSY {
                if unsafe{ WaitNamedPipeW(pipe_name, 10000) } == 0 {
                    return false;
                }
                continue;
            }
            return false;
        }
    }

    fn is_open(&self) -> bool {
        self.handle != INVALID_HANDLE_VALUE
    }

    fn close(&mut self) {
        unsafe { CloseHandle(self.handle) };
        self.handle = INVALID_HANDLE_VALUE;
    }

    fn read(&mut self, buffer: &mut [u8]) -> bool {
        if !self.is_open() {
            return false;
        }
        let mut bytes_available = 0;
        if unsafe { PeekNamedPipe(
            self.handle, ptr::null_mut(), 0, ptr::null_mut(), &mut bytes_available, ptr::null_mut()) } != 0 {
            let mut bytes_read = 0;
            if unsafe { ReadFile(
                self.handle, buffer.as_mut_ptr().cast(), buffer.len() as DWORD, &mut bytes_read, ptr::null_mut()) } != 0 {
                return true;
            }
            else {
                self.close();
            }
        }
        else {
            self.close();
        }
        false
    }

    fn write(&mut self, buffer: &[u8]) -> bool {
        if buffer.len() == 0 {
            return true;
        }
        if !self.is_open() {
            return false;
        }
        let mut bytes_written = 0;
        if unsafe { WriteFile(
            self.handle, buffer.as_ptr().cast(), buffer.len() as DWORD, &mut bytes_written, ptr::null_mut()) } != 0 {
            unsafe{ FlushFileBuffers(self.handle) };
            return bytes_written == buffer.len() as DWORD;
        }
        false
    }
}

impl Drop for NamedPipe {
    fn drop(&mut self) {
        self.close();
    }
}

unsafe impl Send for NamedPipe {}
