use std::{
    ffi::OsString,
    io::{self, BufRead, BufReader, Read},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::mpsc,
    thread,
};
use tracing::trace;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessOutput {
    pub status: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProcessStream {
    Stdout,
    Stderr,
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
    trace!(
        operation = "run_process",
        executable = %executable.display(),
        arg_count = args.len(),
        current_dir = ?current_dir.map(Path::display),
        "external process started"
    );
    let mut command = Command::new(executable);
    command.args(args);
    if let Some(directory) = current_dir {
        command.current_dir(directory);
    }
    let output = command.output()?;
    let process_output = ProcessOutput {
        status: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    };
    trace!(
        operation = "run_process",
        executable = %executable.display(),
        status = ?process_output.status,
        "external process completed"
    );
    Ok(process_output)
}

pub(crate) fn run_streaming<F>(
    executable: &Path,
    args: &[&str],
    current_dir: Option<&Path>,
    mut on_output: F,
) -> io::Result<ProcessOutput>
where
    F: FnMut(ProcessStream, String),
{
    trace!(
        operation = "run_process_streaming",
        executable = %executable.display(),
        arg_count = args.len(),
        current_dir = ?current_dir.map(Path::display),
        "streaming external process started"
    );
    let mut command = Command::new(executable);
    command
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(directory) = current_dir {
        command.current_dir(directory);
    }
    let mut child = command.spawn()?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| io::Error::other("failed to capture process stdout"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| io::Error::other("failed to capture process stderr"))?;
    let (sender, receiver) = mpsc::channel();
    let stdout_sender = sender.clone();
    let stdout_thread =
        thread::spawn(move || read_stream(stdout, ProcessStream::Stdout, stdout_sender));
    let stderr_thread = thread::spawn(move || read_stream(stderr, ProcessStream::Stderr, sender));

    let mut stdout_text = String::new();
    let mut stderr_text = String::new();
    let mut read_error = None;
    for message in receiver {
        match message {
            StreamMessage::Output(stream, text) => {
                match stream {
                    ProcessStream::Stdout => stdout_text.push_str(&text),
                    ProcessStream::Stderr => stderr_text.push_str(&text),
                }
                on_output(stream, text);
            }
            StreamMessage::Error(stream, error) => {
                read_error = Some(format!("{stream:?}: {error}"));
            }
        }
    }
    let status = child.wait()?.code();
    join_stream_thread(stdout_thread)?;
    join_stream_thread(stderr_thread)?;
    if let Some(error) = read_error {
        return Err(io::Error::other(error));
    }
    let process_output = ProcessOutput {
        status,
        stdout: stdout_text,
        stderr: stderr_text,
    };
    trace!(
        operation = "run_process_streaming",
        executable = %executable.display(),
        status = ?process_output.status,
        "streaming external process completed"
    );
    Ok(process_output)
}

enum StreamMessage {
    Output(ProcessStream, String),
    Error(ProcessStream, String),
}

fn read_stream<R: Read>(reader: R, stream: ProcessStream, sender: mpsc::Sender<StreamMessage>) {
    let mut reader = BufReader::new(reader);
    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {
                if sender
                    .send(StreamMessage::Output(stream, line.clone()))
                    .is_err()
                {
                    break;
                }
            }
            Err(error) => {
                let _ = sender.send(StreamMessage::Error(stream, error.to_string()));
                break;
            }
        }
    }
}

fn join_stream_thread(thread: thread::JoinHandle<()>) -> io::Result<()> {
    thread
        .join()
        .map_err(|_| io::Error::other("process output reader thread panicked"))
}

pub(crate) fn open_directory(path: &Path) -> io::Result<()> {
    #[cfg(target_os = "macos")]
    {
        let output = Command::new("/usr/bin/open")
            .args(macos_open_directory_arguments(path))
            .output()?;
        let status = output.status;
        if status.success() {
            Ok(())
        } else {
            let detail = String::from_utf8_lossy(&output.stderr).trim().to_owned();
            let message = if detail.is_empty() {
                format!("open exited with status {status}")
            } else {
                format!("open exited with status {status}: {detail}")
            };
            Err(io::Error::other(message))
        }
    }
    #[cfg(windows)]
    {
        let status = Command::new("explorer.exe").arg(path).status()?;
        if status.success() {
            Ok(())
        } else {
            Err(io::Error::other(format!(
                "explorer.exe exited with status {status}"
            )))
        }
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

#[cfg(target_os = "macos")]
fn macos_open_directory_arguments(path: &Path) -> [OsString; 3] {
    [
        OsString::from("-b"),
        OsString::from("com.apple.finder"),
        path.as_os_str().to_owned(),
    ]
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;

    #[test]
    fn opens_directory_with_finder_bundle_identifier() {
        let path = Path::new("/tmp/Vsedi logs");
        let arguments = macos_open_directory_arguments(path);
        assert_eq!(arguments[0], "-b");
        assert_eq!(arguments[1], "com.apple.finder");
        assert_eq!(arguments[2], path.as_os_str());
    }
}
