use crate::platform::process::{self, ProcessOutput, ProcessStream};
use std::{io, path::Path};

pub(crate) fn run_git(
    executable: &Path,
    args: &[&str],
    current_dir: Option<&Path>,
) -> io::Result<ProcessOutput> {
    process::run(executable, args, current_dir)
}

pub(crate) fn run_git_streaming<F>(
    executable: &Path,
    args: &[&str],
    current_dir: Option<&Path>,
    on_output: F,
) -> io::Result<ProcessOutput>
where
    F: FnMut(ProcessStream, String),
{
    process::run_streaming(executable, args, current_dir, on_output)
}
