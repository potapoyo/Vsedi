# ADR 0001: Use the System Git CLI

- Status: Accepted
- Date: 2026-08-10

## Context

Vsedi needs Git operations on Windows and macOS, including normal repository operations, Git LFS, credentials, and interoperability with the user's existing Git environment.

Possible approaches include embedding a Git implementation/library or invoking the system Git executable.

## Decision

Vsedi will initially use the **system Git CLI from Rust** as its Git backend.

Frontend code will not build or execute arbitrary Git commands. Rust application services will expose explicit operations such as `get_status`, `save_work`, `get_history`, and `restore_preview`.

Commands will be executed with an executable plus structured argument list rather than by concatenating a shell command string.

## Consequences

Positive:

- Git LFS can use its normal Git integration
- existing user Git configuration is respected
- credential helpers and OS-secure stores can be reused
- behavior is close to what advanced users can inspect from a terminal
- fewer semantic differences from normal Git workflows

Negative:

- Git must be installed or bundled/installed later through a separate design
- output parsing must be designed carefully and tested across supported Git versions
- environment/PATH differences must be handled
- process execution is a privileged boundary and must be constrained

## Security constraints

- Do not expose arbitrary command execution to the frontend
- Do not invoke `sh -c`, `cmd /c`, PowerShell command strings, or equivalent for routine Git operations
- Validate the target project path before mutating operations
- Capture exit status/stdout/stderr without logging credentials

## Revisit when

Reconsider a library backend only if system Git proves to create unacceptable installation, portability, or parsing problems.

## References

- Git credentials: https://git-scm.com/docs/gitcredentials.html
- Tauri shell plugin: https://v2.tauri.app/plugin/shell/
