#![allow(unsafe_code)]
//! Win32 named-pipe implementation (byte mode, newline-framed sessions).

use std::ffi::OsStr;
use std::io::{self, Read, Write};
use std::os::windows::ffi::OsStrExt;
use std::os::windows::io::{AsRawHandle, FromRawHandle, IntoRawHandle, OwnedHandle, RawHandle};
use std::ptr;
use std::time::{Duration, Instant};

use crate::{bare_name, PipeSession, PipeTransportError};

#[link(name = "kernel32")]
extern "system" {
    fn CreateNamedPipeW(
        lpName: *const u16,
        dwOpenMode: u32,
        dwPipeMode: u32,
        nMaxInstances: u32,
        nOutBufferSize: u32,
        nInBufferSize: u32,
        nDefaultTimeOut: u32,
        lpSecurityAttributes: *mut core::ffi::c_void,
    ) -> RawHandle;

    fn ConnectNamedPipe(hNamedPipe: RawHandle, lpOverlapped: *mut core::ffi::c_void) -> i32;

    fn DisconnectNamedPipe(hNamedPipe: RawHandle) -> i32;

    fn PeekNamedPipe(
        hNamedPipe: RawHandle,
        lpBuffer: *mut u8,
        nBufferSize: u32,
        lpBytesRead: *mut u32,
        lpTotalBytesAvail: *mut u32,
        lpBytesLeftThisMessage: *mut u32,
    ) -> i32;

    fn GetLastError() -> u32;

    fn SetNamedPipeHandleState(
        hNamedPipe: RawHandle,
        lpMode: *const u32,
        lpMaxCollectionCount: *mut u32,
        lpCollectDataTimeout: *mut u32,
    ) -> i32;
}

const INVALID_HANDLE_VALUE: RawHandle = -1isize as RawHandle;
const PIPE_ACCESS_DUPLEX: u32 = 0x0000_0003;
const PIPE_TYPE_BYTE: u32 = 0x0000_0000;
const PIPE_READMODE_BYTE: u32 = 0x0000_0000;
const PIPE_NOWAIT: u32 = 0x0000_0001;
const PIPE_WAIT: u32 = 0x0000_0000;
const PIPE_UNLIMITED_INSTANCES: u32 = 255;
const ERROR_PIPE_CONNECTED: u32 = 535;
const ERROR_PIPE_LISTENING: u32 = 536;
const ERROR_NO_DATA: u32 = 232;
const ERROR_BROKEN_PIPE: u32 = 109;

fn wide_pipe_path(name: &str) -> Vec<u16> {
    let path = if name.starts_with(r"\\.\pipe\") {
        name.to_string()
    } else {
        format!(r"\\.\pipe\{}", bare_name(name))
    };
    OsStr::new(&path)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

fn last_os_error() -> io::Error {
    io::Error::from_raw_os_error(unsafe { GetLastError() } as i32)
}

pub struct NamedPipeListener {
    pipe_name: String,
}

impl NamedPipeListener {
    pub fn bind(pipe_name: &str) -> Result<Self, PipeTransportError> {
        if bare_name(pipe_name).is_empty() {
            return Err(PipeTransportError::InvalidInput("empty pipe name"));
        }
        Ok(Self {
            pipe_name: pipe_name.to_string(),
        })
    }

    pub fn pipe_name(&self) -> &str {
        &self.pipe_name
    }

    /// Poll until a client connects or `timeout` elapses (PIPE_NOWAIT).
    pub fn accept(&mut self, timeout: Duration) -> Result<NamedPipeStream, PipeTransportError> {
        let wide = wide_pipe_path(&self.pipe_name);
        let handle = unsafe {
            CreateNamedPipeW(
                wide.as_ptr(),
                PIPE_ACCESS_DUPLEX,
                PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_NOWAIT,
                PIPE_UNLIMITED_INSTANCES,
                64 * 1024,
                64 * 1024,
                0,
                ptr::null_mut(),
            )
        };
        if handle == INVALID_HANDLE_VALUE || handle.is_null() {
            return Err(PipeTransportError::Io(last_os_error()));
        }

        let deadline = Instant::now() + timeout;
        loop {
            let connected = unsafe { ConnectNamedPipe(handle, ptr::null_mut()) };
            if connected != 0 {
                break;
            }
            let err = unsafe { GetLastError() };
            if err == ERROR_PIPE_CONNECTED {
                break;
            }
            if err == ERROR_PIPE_LISTENING || err == ERROR_NO_DATA {
                if Instant::now() >= deadline {
                    let _ = unsafe { OwnedHandle::from_raw_handle(handle) };
                    return Err(PipeTransportError::Timeout);
                }
                std::thread::sleep(Duration::from_millis(50));
                continue;
            }
            let _ = unsafe { OwnedHandle::from_raw_handle(handle) };
            return Err(PipeTransportError::Io(io::Error::from_raw_os_error(
                err as i32,
            )));
        }

        // Session I/O: switch off NOWAIT so client writes are readable without spin races.
        let mode: u32 = PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT;
        unsafe {
            let _ = SetNamedPipeHandleState(handle, &mode, ptr::null_mut(), ptr::null_mut());
        }

        Ok(NamedPipeStream {
            handle: unsafe { OwnedHandle::from_raw_handle(handle) },
            read_buf: Vec::new(),
        })
    }
}

pub struct NamedPipeStream {
    handle: OwnedHandle,
    read_buf: Vec<u8>,
}

impl NamedPipeStream {
    fn with_file<R>(&self, f: impl FnOnce(&mut std::fs::File) -> io::Result<R>) -> io::Result<R> {
        let mut file = unsafe { std::fs::File::from_raw_handle(self.handle.as_raw_handle()) };
        let result = f(&mut file);
        let _ = file.into_raw_handle();
        result
    }
}

impl PipeSession for NamedPipeStream {
    fn read_line(&mut self, timeout: Duration) -> Result<String, PipeTransportError> {
        let deadline = Instant::now() + timeout;
        loop {
            if let Some(pos) = self.read_buf.iter().position(|&b| b == b'\n') {
                let mut line: Vec<u8> = self.read_buf.drain(..=pos).collect();
                if line.last() == Some(&b'\n') {
                    line.pop();
                }
                if line.last() == Some(&b'\r') {
                    line.pop();
                }
                return String::from_utf8(line).map_err(|e| {
                    PipeTransportError::Io(io::Error::new(io::ErrorKind::InvalidData, e))
                });
            }

            if Instant::now() >= deadline {
                return Err(PipeTransportError::Timeout);
            }

            let mut avail: u32 = 0;
            let peek_ok = unsafe {
                PeekNamedPipe(
                    self.handle.as_raw_handle(),
                    ptr::null_mut(),
                    0,
                    ptr::null_mut(),
                    &mut avail,
                    ptr::null_mut(),
                )
            };
            if peek_ok == 0 {
                let err = unsafe { GetLastError() };
                if err == ERROR_BROKEN_PIPE || err == ERROR_NO_DATA {
                    return Err(PipeTransportError::Disconnected);
                }
            }

            if avail > 0 {
                let mut tmp = vec![0u8; (avail as usize).clamp(1, 64 * 1024)];
                let n = self
                    .with_file(|f| f.read(&mut tmp))
                    .map_err(PipeTransportError::from)?;
                if n == 0 {
                    return Err(PipeTransportError::Disconnected);
                }
                self.read_buf.extend_from_slice(&tmp[..n]);
            } else {
                std::thread::sleep(Duration::from_millis(20));
            }
        }
    }

    fn write_line(&mut self, line: &str) -> Result<(), PipeTransportError> {
        let mut data = line.as_bytes().to_vec();
        if !data.ends_with(b"\n") {
            data.push(b'\n');
        }
        self.with_file(|f| {
            f.write_all(&data)?;
            f.flush()?;
            Ok(())
        })
        .map_err(PipeTransportError::from)
    }

    fn close(&mut self) {
        unsafe {
            let _ = DisconnectNamedPipe(self.handle.as_raw_handle());
        }
    }
}

impl Drop for NamedPipeStream {
    fn drop(&mut self) {
        self.close();
    }
}
