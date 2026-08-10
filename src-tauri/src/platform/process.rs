use std::{
    ffi::OsString,
    io,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessOutput {
    pub status: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

pub fn find_executable(name: &str) -> Option<PathBuf> {
    let requested = Path::new(name);
    if requested.components().count() > 1 {
        return requested.is_file().then(|| requested.to_path_buf());
    }

    let path = std::env::var_os("PATH")?;
    let candidates = executable_candidates(name);
    for directory in std::env::split_paths(&path) {
        for candidate in &candidates {
            let executable = directory.join(candidate);
            if executable.is_file() {
                return Some(executable);
            }
        }
    }
    None
}

fn executable_candidates(name: &str) -> Vec<OsString> {
    #[cfg(windows)]
    {
        let lower = name.to_ascii_lowercase();
        if lower.ends_with(".exe") || lower.ends_with(".cmd") || lower.ends_with(".bat") {
            vec![OsString::from(name)]
        } else {
            vec![OsString::from(name), OsString::from(format!("{name}.exe"))]
        }
    }
    #[cfg(not(windows))]
    {
        vec![OsString::from(name)]
    }
}

pub(crate) fn run(
    executable: &Path,
    args: &[&str],
    current_dir: Option<&Path>,
) -> io::Result<ProcessOutput> {
    let mut command = Command::new(executable);
    command.args(args);
    if let Some(directory) = current_dir {
        command.current_dir(directory);
    }
    let output = command.output()?;
    Ok(ProcessOutput {
        status: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    })
}

pub(crate) fn open_directory(path: &Path) -> io::Result<()> {
    #[cfg(target_os = "macos")]
    {
        Command::new("open").arg(path).status()?;
        return Ok(());
    }
    #[cfg(windows)]
    {
        Command::new("explorer.exe").arg(path).status()?;
        return Ok(());
    }
    #[cfg(not(any(target_os = "macos", windows)))]
    {
        let _ = path;
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "opening a directory is unsupported on this platform",
        ))
    }
}
