# Architecture

## Overview

Vsedi is a Tauri v2 desktop application for Windows and macOS.

The application is split so that the web frontend presents state and user intent, while Rust owns filesystem access, Git process execution, project validation, and other privileged operations.

```text
Frontend UI
    |
    | typed Tauri commands / events
    v
Tauri Command Boundary
    |
    v
Application Services
    |-- ProjectService
    |-- DiagnosticsService
    |-- SaveService
    |-- HistoryService
    |-- RestoreService
    `-- SyncService          (post-Core)
    |
    v
Domain / Adapters
    |-- GitAdapter
    |-- UnityProjectAnalyzer
    |-- VrchatProjectAnalyzer
    |-- LfsAnalyzer
    |-- FileSafety
    `-- PlatformAdapter
```

## Frontend responsibilities

The frontend may:

- render project lists, diagnostics, changes, history, and previews
- collect user intent and confirmation
- request a predefined application operation
- show structured progress / errors returned by Rust

The frontend must not:

- execute arbitrary shell commands
- construct raw Git commands for execution
- directly mutate arbitrary files in a project
- receive or persist Git passwords/tokens as application configuration

## Rust command boundary

Tauri commands should model application intent rather than Git syntax.

Prefer commands similar to:

- `inspect_environment()`
- `inspect_project(path)`
- `initialize_project(path, plan)`
- `get_worktree_status(project_id)`
- `save_work(project_id, message)`
- `get_history(project_id, cursor)`
- `get_revision_detail(project_id, revision)`
- `preview_restore(project_id, revision)`
- `restore_revision(project_id, revision, confirmation)`

Avoid a generic command such as:

- `run_git(args)`
- `run_shell(command)`

A generic escape hatch would move the security boundary into the frontend and defeat the purpose of the service layer.

## Application services

### ProjectService

Owns registered-project identity, path normalization, and project lifecycle.

### DiagnosticsService

Combines Git, Unity, VRChat/VPM, LFS, ignore, and large-file diagnostics into user-facing findings.

### SaveService

Turns the product concept of "作業を保存" into validated Git index/commit operations.

### HistoryService

Reads commit history and revision details without mutating the repository.

### RestoreService

Owns restore preview, safety snapshot creation, restore execution, validation, and recovery metadata.

### SyncService

Added after Vsedi Core. Owns fetch/push/fast-forward synchronization and divergence detection. It must follow ADR 0003 and stop on diverged history.

## GitAdapter

The initial adapter uses the system Git CLI per ADR 0001.

Requirements:

- executable and arguments are passed separately
- working directory is explicitly provided
- locale-sensitive human output should be avoided where machine-readable formats exist
- exit code, stdout, and stderr are captured separately
- parsers have fixture tests
- secrets are not copied into logs

Useful Git plumbing/porcelain formats should be selected during implementation based on stable machine-readable output, such as NUL-delimited status formats where appropriate.

## Project identity and paths

A project operation must start from a registered, normalized project root.

Before mutation:

1. resolve the registered project root
2. canonicalize/normalize the target path according to platform rules
3. confirm the operation is scoped to the intended repository
4. reject unexpected repository/worktree boundaries unless explicitly supported

Symlinks, junctions, nested repositories, and worktrees need tests before being treated as supported scenarios.

## State storage

Application-level preferences may eventually use a small local store for information such as:

- onboarding completion
- recent project paths
- non-secret UI preferences

Repository truth must remain in Git / project files rather than being duplicated as authoritative application state.

Secrets must not be stored in the ordinary preference store.

## Error model

Rust operations should return structured errors with at least:

- stable application error code
- safe user-facing summary
- optional technical detail safe for local display
- operation stage
- whether repository mutation may have occurred

Raw stderr may be useful for diagnostics but should not automatically be treated as a user-friendly message or exported without redaction review.

## Platform boundary

Windows/macOS-specific behavior belongs behind adapters, especially:

- detecting/running Unity
- opening Finder / Explorer
- executable discovery
- process inspection
- path behavior

Core Git and domain logic should be platform-independent where possible.

## Tauri capabilities

Use the minimum required Tauri permissions. Prefer Rust-owned process execution over exposing broad shell execution to JavaScript.

Any capability addition that permits general process/filesystem access should receive explicit security review.

## Deferred architecture decisions

The following are intentionally not fixed in M0:

- frontend framework and UI component library
- exact application preference store
- exact restore/safety-snapshot mechanism
- GitHub-specific OAuth
- updater / signing infrastructure

These should be decided when their milestone begins and recorded as ADRs when they materially constrain future design.

## References

- Tauri v2 Shell: https://v2.tauri.app/plugin/shell/
- Git credentials: https://git-scm.com/docs/gitcredentials.html
- VRChat VPM source control: https://vcc.docs.vrchat.com/vpm/source-control/
