use std::{ffi::OsStr, mem::size_of, os::windows::ffi::OsStrExt};

use windows::{
    core::PWSTR,
    Win32::{
        Foundation::BOOL,
        Security::SECURITY_ATTRIBUTES,
        System::Threading::{
            CreateProcessW, InitializeProcThreadAttributeList, UpdateProcThreadAttribute,
            EXTENDED_STARTUPINFO_PRESENT, LPPROC_THREAD_ATTRIBUTE_LIST, PROCESS_INFORMATION,
            STARTUPINFOEXW,
        },
    },
};

pub struct ProcessFactory {
    pub startup_info: STARTUPINFOEXW,
    pub process_info: PROCESS_INFORMATION,
}

impl ProcessFactory {
    pub unsafe fn start(
        command: String,
        lpvalue: *mut std::ffi::c_void,
        attributes: usize,
    ) -> Self {
        let startup_info = Self::configure_process_thread(lpvalue, attributes);
        let process_info = Self::run(startup_info, command);

        Self {
            startup_info,
            process_info,
        }
    }
    pub unsafe fn configure_process_thread(
        lpvalue: *mut std::ffi::c_void,
        attributes: usize,
    ) -> STARTUPINFOEXW {
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
            attributes,
            Some(lpvalue), // TODO
            size_of::<*mut std::ffi::c_void>(),
            None,
            None,
        )
        .unwrap();

        return startup_info;
    }

    pub unsafe fn run(startup_info: STARTUPINFOEXW, command: String) -> PROCESS_INFORMATION {
        let mut pi_client = PROCESS_INFORMATION::default();

        let mut cmd: Vec<u16> = OsStr::new(&command).encode_wide().collect();
        cmd.push(0);

        let cmd_ptr = PWSTR(cmd.as_mut_ptr().cast());

        let security_attr_size = std::mem::size_of::<SECURITY_ATTRIBUTES>();

        let p_sec = SECURITY_ATTRIBUTES {
            nLength: security_attr_size as u32,
            lpSecurityDescriptor: std::ptr::null_mut(),
            bInheritHandle: BOOL(0),
        };

        let t_sec = SECURITY_ATTRIBUTES {
            nLength: security_attr_size as u32,
            lpSecurityDescriptor: std::ptr::null_mut(),
            bInheritHandle: BOOL(0),
        };

        CreateProcessW(
            None,
            cmd_ptr,
            Some(&p_sec),
            Some(&t_sec),
            false,
            EXTENDED_STARTUPINFO_PRESENT,
            None,
            None,
            &startup_info.StartupInfo,
            &mut pi_client as *mut PROCESS_INFORMATION,
        )
        .unwrap();

        return pi_client;
    }
}
