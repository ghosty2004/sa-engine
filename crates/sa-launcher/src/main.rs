//! sa-launcher: starts gta_sa.exe suspended, loads sa_hook.dll into it with a remote `LoadLibraryW`
//! thread (DllMain installs the hooks while the game's main thread is still suspended), then resumes
//! the game and waits for it.
//!
//! Usage: sa-launcher [--gta PATH] [--dll PATH]... [--no-wait] [-- GAME ARGS...]
//!
//! Without `--dll`, `sa_hook.dll` next to the launcher is loaded. Log: stdout and `sa-launcher.log`
//! next to gta_sa.exe.

#[derive(Debug, Clone, PartialEq, Eq)]
struct Options {
    gta: String,
    dlls: Vec<String>,
    wait: bool,
    game_args: Vec<String>,
}

const USAGE: &str = "usage: sa-launcher [--gta PATH] [--dll PATH]... [--no-wait] [-- GAME ARGS...]";

fn parse_args<I: IntoIterator<Item = String>>(args: I) -> Result<Options, String> {
    let mut o = Options {
        gta: "gta_sa.exe".into(),
        dlls: Vec::new(),
        wait: true,
        game_args: Vec::new(),
    };
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        let mut val = |flag: &str| it.next().ok_or_else(|| format!("{flag} needs a value"));
        match a.as_str() {
            "--gta" => o.gta = val("--gta")?,
            "--dll" => o.dlls.push(val("--dll")?),
            "--no-wait" => o.wait = false,
            "--" => {
                o.game_args.extend(it.by_ref());
                break;
            }
            "--help" | "-?" => return Err(USAGE.into()),
            other => return Err(format!("unknown argument {other:?}\n{USAGE}")),
        }
    }
    Ok(o)
}

/// Quotes one argument for a Windows command line (CommandLineToArgvW rules).
fn quote_arg(a: &str) -> String {
    if !a.is_empty() && !a.contains([' ', '\t', '"']) {
        return a.to_string();
    }
    let mut out = String::from('"');
    let mut backslashes = 0;
    for c in a.chars() {
        match c {
            '\\' => backslashes += 1,
            '"' => {
                out.push_str(&"\\".repeat(backslashes * 2 + 1));
                out.push('"');
                backslashes = 0;
            }
            _ => {
                out.push_str(&"\\".repeat(backslashes));
                out.push(c);
                backslashes = 0;
            }
        }
    }
    out.push_str(&"\\".repeat(backslashes * 2));
    out.push('"');
    out
}

fn command_line(o: &Options) -> String {
    std::iter::once(&o.gta)
        .chain(&o.game_args)
        .map(|a| quote_arg(a))
        .collect::<Vec<_>>()
        .join(" ")
}

fn main() {
    let opts = match parse_args(std::env::args().skip(1)) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(2);
        }
    };
    std::process::exit(run(&opts));
}

#[cfg(not(windows))]
fn run(o: &Options) -> i32 {
    eprintln!(
        "sa-launcher only runs on Windows (or Wine); would run: {}",
        command_line(o)
    );
    1
}

#[cfg(windows)]
fn run(o: &Options) -> i32 {
    match win::launch(o) {
        Ok(code) => code,
        Err(e) => {
            win::log(&format!("error: {e}"));
            1
        }
    }
}

#[cfg(windows)]
mod win {
    use super::{Options, command_line};
    use std::ffi::c_void;
    use std::io::Write;
    use std::os::windows::ffi::OsStrExt;
    use std::path::{Path, PathBuf};
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0};
    use windows_sys::Win32::System::Diagnostics::Debug::WriteProcessMemory;
    use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
    use windows_sys::Win32::System::Memory::{
        MEM_COMMIT, MEM_RELEASE, MEM_RESERVE, PAGE_READWRITE, VirtualAllocEx, VirtualFreeEx,
    };
    use windows_sys::Win32::System::Threading::{
        CREATE_SUSPENDED, CreateProcessW, CreateRemoteThread, GetExitCodeProcess, GetExitCodeThread,
        INFINITE, PROCESS_INFORMATION, ResumeThread, STARTUPINFOW, TerminateProcess, WaitForSingleObject,
    };

    static LOG_PATH: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();

    pub fn log(msg: &str) {
        println!("[sa-launcher] {msg}");
        if let Some(p) = LOG_PATH.get()
            && let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(p)
        {
            let _ = writeln!(f, "{msg}");
        }
    }

    fn wide(s: &std::ffi::OsStr) -> Vec<u16> {
        s.encode_wide().chain(std::iter::once(0)).collect()
    }

    struct Handle(HANDLE);
    impl Drop for Handle {
        fn drop(&mut self) {
            if !self.0.is_null() {
                // SAFETY: we own the handle.
                unsafe { CloseHandle(self.0) };
            }
        }
    }

    fn last_error(what: &str) -> String {
        format!("{what} failed: {}", std::io::Error::last_os_error())
    }

    pub fn launch(o: &Options) -> Result<i32, String> {
        let gta = std::path::absolute(&o.gta).map_err(|e| format!("bad --gta path: {e}"))?;
        if !gta.is_file() {
            return Err(format!("gta_sa.exe not found at {}", gta.display()));
        }
        let gta_dir = gta.parent().unwrap_or(Path::new(".")).to_path_buf();
        let _ = std::fs::remove_file(gta_dir.join("sa-launcher.log"));
        let _ = LOG_PATH.set(gta_dir.join("sa-launcher.log"));
        let mut dlls = Vec::new();
        for d in &o.dlls {
            dlls.push(std::path::absolute(d).map_err(|e| format!("bad --dll path: {e}"))?);
        }
        if dlls.is_empty() {
            let exe = std::env::current_exe().map_err(|e| e.to_string())?;
            dlls.push(exe.parent().unwrap_or(Path::new(".")).join("sa_hook.dll"));
        }
        for dll in &dlls {
            if !dll.is_file() {
                return Err(format!("DLL not found at {}", dll.display()));
            }
        }
        let mut opts = o.clone();
        opts.gta = gta.to_string_lossy().into_owned();
        let cmd = command_line(&opts);
        log(&format!("starting (suspended): {cmd}"));

        let app = wide(gta.as_os_str());
        let mut cmd_w = wide(std::ffi::OsStr::new(&cmd));
        let dir_w = wide(gta_dir.as_os_str());
        // SAFETY: Win32 process creation with valid, NUL-terminated buffers.
        let pi = unsafe {
            let mut si: STARTUPINFOW = std::mem::zeroed();
            si.cb = std::mem::size_of::<STARTUPINFOW>() as u32;
            let mut pi: PROCESS_INFORMATION = std::mem::zeroed();
            if CreateProcessW(
                app.as_ptr(),
                cmd_w.as_mut_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                0,
                CREATE_SUSPENDED,
                std::ptr::null(),
                dir_w.as_ptr(),
                &si,
                &mut pi,
            ) == 0
            {
                return Err(last_error("CreateProcessW"));
            }
            pi
        };
        let process = Handle(pi.hProcess);
        let thread = Handle(pi.hThread);
        log(&format!("gta_sa.exe pid {}", pi.dwProcessId));

        for dll in &dlls {
            if let Err(e) = inject(process.0, dll) {
                // SAFETY: killing the child we just created.
                unsafe { TerminateProcess(process.0, 1) };
                return Err(e);
            }
        }
        // SAFETY: resuming the suspended main thread of our child.
        if unsafe { ResumeThread(thread.0) } == u32::MAX {
            unsafe { TerminateProcess(process.0, 1) };
            return Err(last_error("ResumeThread"));
        }
        log("game resumed");
        if !o.wait {
            return Ok(0);
        }
        // SAFETY: waiting on our own child handle.
        unsafe {
            WaitForSingleObject(process.0, INFINITE);
            let mut code = 0u32;
            GetExitCodeProcess(process.0, &mut code);
            log(&format!("gta_sa.exe exited with code {code:#x}"));
            Ok(code as i32)
        }
    }

    /// Remote-thread injection: write the DLL path into the target and run `LoadLibraryW(path)` there
    /// (kernel32 is mapped at the same address in every process of a session).
    fn inject(process: HANDLE, dll: &Path) -> Result<(), String> {
        log(&format!("injecting: {}", dll.display()));
        let path_w = wide(dll.as_os_str());
        let bytes = path_w.len() * 2;
        // SAFETY: Win32 calls on a process handle we own; buffers are valid.
        unsafe {
            let k32 = GetModuleHandleW(wide(std::ffi::OsStr::new("kernel32.dll")).as_ptr());
            let load =
                GetProcAddress(k32, c"LoadLibraryW".as_ptr().cast()).ok_or("LoadLibraryW not found")?;
            let remote = VirtualAllocEx(
                process,
                std::ptr::null(),
                bytes,
                MEM_COMMIT | MEM_RESERVE,
                PAGE_READWRITE,
            );
            if remote.is_null() {
                return Err(last_error("VirtualAllocEx"));
            }
            let mut written = 0usize;
            if WriteProcessMemory(process, remote, path_w.as_ptr().cast(), bytes, &mut written) == 0
                || written != bytes
            {
                return Err(last_error("WriteProcessMemory"));
            }
            let start: unsafe extern "system" fn(*mut c_void) -> u32 = std::mem::transmute(load);
            let th = CreateRemoteThread(
                process,
                std::ptr::null(),
                0,
                Some(start),
                remote,
                0,
                std::ptr::null_mut(),
            );
            if th.is_null() {
                return Err(last_error("CreateRemoteThread"));
            }
            let th = Handle(th);
            if WaitForSingleObject(th.0, 60_000) != WAIT_OBJECT_0 {
                return Err("LoadLibraryW in the game did not finish within 60 s".into());
            }
            let mut module = 0u32;
            GetExitCodeThread(th.0, &mut module);
            VirtualFreeEx(process, remote, 0, MEM_RELEASE);
            if module == 0 {
                return Err(format!("LoadLibraryW({}) failed inside the game", dll.display()));
            }
            log(&format!("loaded at {module:#x}: {}", dll.display()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn args() {
        let o = parse_args(s(&[])).unwrap();
        assert_eq!((o.gta.as_str(), o.wait, o.dlls.len()), ("gta_sa.exe", true, 0));
        let o = parse_args(s(&[
            "--gta",
            "C:\\G A\\gta_sa.exe",
            "--dll",
            "a.dll",
            "--dll",
            "b.dll",
            "--no-wait",
        ]))
        .unwrap();
        assert_eq!(o.dlls, s(&["a.dll", "b.dll"]));
        assert!(!o.wait);
        assert_eq!(command_line(&o), "\"C:\\G A\\gta_sa.exe\"");
        let o = parse_args(s(&["--", "-x", "two words"])).unwrap();
        assert_eq!(command_line(&o), "gta_sa.exe -x \"two words\"");
        assert!(parse_args(s(&["--bogus"])).is_err());
        assert!(parse_args(s(&["--gta"])).is_err());
    }

    #[test]
    fn quoting() {
        assert_eq!(quote_arg("plain"), "plain");
        assert_eq!(quote_arg(""), "\"\"");
        assert_eq!(quote_arg("a\"b"), "\"a\\\"b\"");
        assert_eq!(quote_arg("C:\\dir with space\\"), "\"C:\\dir with space\\\\\"");
    }
}
