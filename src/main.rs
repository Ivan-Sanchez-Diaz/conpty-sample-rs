use std::ffi::OsStr;
use std::mem::size_of;
use std::os::windows::prelude::OsStrExt;

use windows::core::PWSTR;
use windows::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
use windows::Win32::Storage::FileSystem::{ReadFile, WriteFile};
use windows::Win32::System::Console::{
    ClosePseudoConsole, CreatePseudoConsole, GetConsoleScreenBufferInfo, HPCON,
};
use windows::Win32::System::Console::{GetConsoleMode, GetStdHandle, CONSOLE_MODE};
use windows::Win32::System::Console::{CONSOLE_SCREEN_BUFFER_INFO, COORD, STD_OUTPUT_HANDLE};
use windows::Win32::System::Pipes::CreatePipe;
use windows::Win32::System::Threading::{
    CreateProcessW, DeleteProcThreadAttributeList, InitializeProcThreadAttributeList, Sleep,
    UpdateProcThreadAttribute, WaitForSingleObject, EXTENDED_STARTUPINFO_PRESENT,
    LPPROC_THREAD_ATTRIBUTE_LIST, PROCESS_INFORMATION, PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE,
    STARTUPINFOEXW,
};

fn main() {
    unsafe { run() }
}

pub unsafe fn run() {
    let h_console = GetStdHandle(STD_OUTPUT_HANDLE).unwrap();
    let mut console_mode = CONSOLE_MODE::default();
    GetConsoleMode(h_console, &mut console_mode as *mut CONSOLE_MODE).unwrap();

    let mut pipe_in = INVALID_HANDLE_VALUE;
    let mut pipe_out = INVALID_HANDLE_VALUE;

    let mut pipe_pty_in = INVALID_HANDLE_VALUE;
    let mut pipe_pty_out = INVALID_HANDLE_VALUE;

    CreatePipe(
        &mut pipe_pty_in as *mut HANDLE,
        &mut pipe_out as *mut HANDLE,
        None,
        0,
    )
    .unwrap();

    CreatePipe(
        &mut pipe_in as *mut HANDLE,
        &mut pipe_pty_out as *mut HANDLE,
        None,
        0,
    )
    .unwrap();

    let mut console_size = COORD::default();
    let mut csbi = CONSOLE_SCREEN_BUFFER_INFO::default();

    if GetConsoleScreenBufferInfo(h_console, &mut csbi).is_ok() {
        console_size.X = csbi.srWindow.Right - csbi.srWindow.Left + 1;
        console_size.Y = csbi.srWindow.Bottom - csbi.srWindow.Top + 1;
    }

    let h_pc = CreatePseudoConsole(console_size, pipe_pty_in, pipe_pty_out, 0).unwrap();

    if INVALID_HANDLE_VALUE != pipe_pty_out {
        CloseHandle(pipe_pty_out).unwrap();
        eprintln!("CLOSE");
    }

    if INVALID_HANDLE_VALUE != pipe_pty_in {
        CloseHandle(pipe_pty_in).unwrap();
        eprintln!("CLOSE");
    }

    // TODO: Listen thread
    std::thread::spawn(move || {
        println!("hello, world!");
        let h_console = GetStdHandle(STD_OUTPUT_HANDLE).unwrap();

        let mut bytes_read = 0;
        let mut bytes_write = 0;
        let mut buffer = vec![0u8; 512];

        ReadFile(pipe_in, Some(&mut buffer), Some(&mut bytes_read), None).unwrap();
        WriteFile(h_console, Some(&mut buffer), Some(&mut bytes_write), None).unwrap();

        while bytes_read > 0 {
            ReadFile(pipe_in, Some(&mut buffer), Some(&mut bytes_read), None).unwrap();
            WriteFile(h_console, Some(&mut buffer), Some(&mut bytes_write), None).unwrap();
        }
    });

    let mut startup_info = STARTUPINFOEXW::default();

    let mut attr_list_size = 0;
    startup_info.StartupInfo.cb = size_of::<STARTUPINFOEXW>() as u32;

    InitializeProcThreadAttributeList(
        LPPROC_THREAD_ATTRIBUTE_LIST(std::ptr::null_mut()),
        1,
        0,
        &mut attr_list_size,
    )
    .ok();

    let inner = vec![0u8; attr_list_size].into_boxed_slice();
    let inner = Box::leak(inner);

    startup_info.lpAttributeList = LPPROC_THREAD_ATTRIBUTE_LIST(inner.as_mut_ptr().cast());

    InitializeProcThreadAttributeList(startup_info.lpAttributeList, 1, 0, &mut attr_list_size)
        .unwrap();

    UpdateProcThreadAttribute(
        startup_info.lpAttributeList,
        0,
        PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE.try_into().unwrap(),
        Some(h_pc.0 as _), // TODO
        size_of::<HPCON>(),
        None,
        None,
    )
    .unwrap();

    let mut pi_client = PROCESS_INFORMATION::default();

    let mut cmd: Vec<u16> = OsStr::new("ping localhost").encode_wide().collect();
    cmd.push(0);

    let cmd_ptr = PWSTR(cmd.as_mut_ptr().cast());

    CreateProcessW(
        None,
        cmd_ptr,
        None,
        None,
        false,
        EXTENDED_STARTUPINFO_PRESENT,
        None,
        None,
        &startup_info.StartupInfo,
        &mut pi_client as *mut PROCESS_INFORMATION,
    )
    .unwrap();

    WaitForSingleObject(pi_client.hThread, 10 * 1000);
    Sleep(500);

    CloseHandle(pi_client.hThread).unwrap();
    CloseHandle(pi_client.hProcess).unwrap();

    DeleteProcThreadAttributeList(startup_info.lpAttributeList);
    ClosePseudoConsole(h_pc);

    if pipe_in != INVALID_HANDLE_VALUE {
        CloseHandle(pipe_in).unwrap();
    }

    if pipe_out != INVALID_HANDLE_VALUE {
        CloseHandle(pipe_out).unwrap();
    }
}
