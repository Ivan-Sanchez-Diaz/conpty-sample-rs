use std::{
    io::{Read, Write},
    os::windows::io::FromRawHandle,
};

use windows::Win32::{
    Foundation::{HANDLE, INVALID_HANDLE_VALUE},
    System::{
        Console::{
            CreatePseudoConsole, GetConsoleMode, GetConsoleScreenBufferInfo, GetStdHandle,
            SetConsoleMode, CONSOLE_MODE, CONSOLE_SCREEN_BUFFER_INFO, COORD,
            DISABLE_NEWLINE_AUTO_RETURN, ENABLE_VIRTUAL_TERMINAL_PROCESSING, HPCON,
            STD_OUTPUT_HANDLE,
        },
        Pipes::CreatePipe,
        Threading::PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE,
    },
};

use crate::process::ProcessFactory;

pub struct Terminal;

impl Terminal {
    pub(crate) unsafe fn enable_vt_terminal_sequence_proccesing() {
        let h_stdout = GetStdHandle(STD_OUTPUT_HANDLE).unwrap();
        let mut console_mode = CONSOLE_MODE::default();
        GetConsoleMode(h_stdout, &mut console_mode as *mut CONSOLE_MODE).unwrap();

        console_mode =
            console_mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING | DISABLE_NEWLINE_AUTO_RETURN;

        SetConsoleMode(h_stdout, console_mode).unwrap();
    }

    pub unsafe fn run(command: &str) {
        Self::enable_vt_terminal_sequence_proccesing();

        let mut input_pipe = PseudoConsolePipe::new();
        let mut output_pipe = PseudoConsolePipe::new();

        let h_pc = Self::create_pseudo_console_and_pipes(
            &mut input_pipe.read_side,
            &mut output_pipe.write_side,
        );

        let _proc = ProcessFactory::start(
            command.to_string(),
            h_pc.0 as _,
            PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE.try_into().unwrap(),
        );

        let handle = std::thread::spawn(move || {
            let mut buffer = vec![0u8; 512];

            let mut file =
                std::fs::File::from_raw_handle(output_pipe.read_side.0 as *mut std::ffi::c_void);

            loop {
                let Ok(bytes) = file.read(&mut buffer[..]) else {
                    break;
                };

                std::io::stdout().write_all(&buffer[..bytes]).unwrap();
                std::io::stdout().write_all(b"\n").unwrap();
            }
        });

        std::thread::spawn(move || {
            let mut buffer = String::new();

            let mut file =
                std::fs::File::from_raw_handle(input_pipe.write_side.0 as *mut std::ffi::c_void);

            loop {
                std::io::stdin().read_line(&mut buffer).unwrap();
                file.write_all(buffer.as_bytes()).unwrap();
            }
        });

        handle.join().unwrap();
    }

    pub unsafe fn create_pseudo_console_and_pipes(
        pipe_in: &mut HANDLE,
        pipe_out: &mut HANDLE,
    ) -> HPCON {
        let mut console_size = COORD::default();
        let mut csbi = CONSOLE_SCREEN_BUFFER_INFO::default();

        let h_console = GetStdHandle(STD_OUTPUT_HANDLE).unwrap();

        if GetConsoleScreenBufferInfo(h_console, &mut csbi).is_ok() {
            console_size.X = csbi.srWindow.Right - csbi.srWindow.Left + 1;
            console_size.Y = csbi.srWindow.Bottom - csbi.srWindow.Top + 1;
        }

        let h_pc = CreatePseudoConsole(console_size, *pipe_in, *pipe_out, 0).unwrap();

        return h_pc;
    }
}

pub struct PseudoConsolePipe {
    pub read_side: HANDLE,
    pub write_side: HANDLE,
}

impl PseudoConsolePipe {
    pub unsafe fn new() -> Self {
        let mut read_side = INVALID_HANDLE_VALUE;
        let mut write_side = INVALID_HANDLE_VALUE;

        unsafe {
            CreatePipe(
                &mut read_side as *mut HANDLE,
                &mut write_side as *mut HANDLE,
                None,
                0,
            )
            .unwrap()
        };

        Self {
            read_side,
            write_side,
        }
    }
}
