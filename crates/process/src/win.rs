//! Windows real-time process path / PID resolution via Toolhelp + QueryFullProcessImageName.

#![cfg(windows)]
#![allow(unsafe_code)]

use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::path::PathBuf;

use crate::identity::ProcessIdentity;
use crate::discovery::ProcessSnapshot;

type HANDLE = *mut core::ffi::c_void;
type DWORD = u32;
type BOOL = i32;

const TH32CS_SNAPPROCESS: DWORD = 0x00000002;
const PROCESS_QUERY_LIMITED_INFORMATION: DWORD = 0x1000;
const INVALID_HANDLE_VALUE: HANDLE = -1isize as HANDLE;

#[repr(C)]
struct ProcessEntry32W {
    dw_size: DWORD,
    cnt_usage: DWORD,
    th32_process_id: DWORD,
    th32_default_heap_id: usize,
    th32_module_id: DWORD,
    cnt_threads: DWORD,
    th32_parent_process_id: DWORD,
    pc_pri_class_base: i32,
    dw_flags: DWORD,
    sz_exe_file: [u16; 260],
}

#[link(name = "kernel32")]
extern "system" {
    fn CreateToolhelp32Snapshot(flags: DWORD, process_id: DWORD) -> HANDLE;
    fn Process32FirstW(snapshot: HANDLE, entry: *mut ProcessEntry32W) -> BOOL;
    fn Process32NextW(snapshot: HANDLE, entry: *mut ProcessEntry32W) -> BOOL;
    fn CloseHandle(handle: HANDLE) -> BOOL;
    fn OpenProcess(access: DWORD, inherit: BOOL, pid: DWORD) -> HANDLE;
    fn QueryFullProcessImageNameW(
        process: HANDLE,
        flags: DWORD,
        buf: *mut u16,
        size: *mut DWORD,
    ) -> BOOL;
    fn GetLastError() -> DWORD;
}

fn wide_to_string(buf: &[u16]) -> String {
    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    OsString::from_wide(&buf[..len])
        .to_string_lossy()
        .into_owned()
}

/// Resolve full image path for a PID. Falls back to exe name from Toolhelp if path query fails.
pub fn resolve_pid_path(pid: u32) -> Option<ProcessSnapshot> {
    if pid == 0 {
        return None;
    }
    let path = query_full_image_name(pid);
    let exe_name = path
        .as_ref()
        .and_then(|p| {
            PathBuf::from(p)
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
        })
        .or_else(|| toolhelp_exe_name(pid));

    let identity = if let Some(ref full) = path {
        ProcessIdentity::from_path(full)
    } else if let Some(ref name) = exe_name {
        ProcessIdentity::from_path(name)
    } else {
        return None;
    };

    Some(ProcessSnapshot {
        pid,
        identity,
    })
}

fn query_full_image_name(pid: u32) -> Option<String> {
    unsafe {
        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if h.is_null() || h == INVALID_HANDLE_VALUE {
            let _ = GetLastError();
            return None;
        }
        let mut buf = vec![0u16; 1024];
        let mut size = buf.len() as DWORD;
        let ok = QueryFullProcessImageNameW(h, 0, buf.as_mut_ptr(), &mut size);
        CloseHandle(h);
        if ok == 0 {
            return None;
        }
        Some(wide_to_string(&buf[..size as usize]))
    }
}

fn toolhelp_exe_name(pid: u32) -> Option<String> {
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snap.is_null() || snap == INVALID_HANDLE_VALUE {
            return None;
        }
        let mut entry = std::mem::zeroed::<ProcessEntry32W>();
        entry.dw_size = std::mem::size_of::<ProcessEntry32W>() as DWORD;
        let mut found = None;
        if Process32FirstW(snap, &mut entry) != 0 {
            loop {
                if entry.th32_process_id == pid {
                    found = Some(wide_to_string(&entry.sz_exe_file));
                    break;
                }
                if Process32NextW(snap, &mut entry) == 0 {
                    break;
                }
            }
        }
        CloseHandle(snap);
        found
    }
}

/// Enumerate all processes with best-effort full paths.
pub fn list_processes() -> Vec<ProcessSnapshot> {
    let mut out = Vec::new();
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snap.is_null() || snap == INVALID_HANDLE_VALUE {
            return out;
        }
        let mut entry = std::mem::zeroed::<ProcessEntry32W>();
        entry.dw_size = std::mem::size_of::<ProcessEntry32W>() as DWORD;
        if Process32FirstW(snap, &mut entry) != 0 {
            loop {
                let pid = entry.th32_process_id;
                if pid != 0 {
                    if let Some(snap) = resolve_pid_path(pid) {
                        out.push(snap);
                    } else {
                        let name = wide_to_string(&entry.sz_exe_file);
                        out.push(ProcessSnapshot {
                            pid,
                            identity: ProcessIdentity::from_path(&name),
                        });
                    }
                }
                if Process32NextW(snap, &mut entry) == 0 {
                    break;
                }
            }
        }
        CloseHandle(snap);
    }
    out
}
