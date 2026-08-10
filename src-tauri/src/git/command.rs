use crate::platform::process::{self, ProcessOutput};
use std::{io, path::Path};

pub(crate) fn run_git(
    executable: &Path,
    args: &[&str],
    current_dir: Option<&Path>,
) -> io::Result<ProcessOutput> {
    process::run(executable, args, current_dir)
}
