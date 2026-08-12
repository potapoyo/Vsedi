use crate::models::{ChangeKind, ChangedFile, WorktreeSnapshot};

/// Parses `git status --porcelain=v2 -z --untracked-files=all` without relying
/// on Git's localized human-readable output.
pub fn parse_porcelain_v2(
    output: &str,
    project_prefix: Option<&str>,
) -> Result<WorktreeSnapshot, String> {
    let entries = output
        .split('\0')
        .filter(|entry| !entry.is_empty())
        .collect::<Vec<_>>();
    let mut files = Vec::new();
    let mut index = 0;
    while index < entries.len() {
        let entry = entries[index];
        let record = entry
            .split_once(' ')
            .map(|(record, _)| record)
            .unwrap_or(entry);
        match record {
            "1" => {
                let mut fields = entry.splitn(9, ' ');
                fields.next();
                let xy = fields.next().ok_or("ordinary record has no XY status")?;
                for _ in 0..6 {
                    fields.next().ok_or("ordinary record is incomplete")?;
                }
                let path = fields.next().ok_or("ordinary record has no path")?;
                files.push(changed(path, None, xy, project_prefix));
            }
            "2" => {
                let mut fields = entry.splitn(10, ' ');
                fields.next();
                let xy = fields.next().ok_or("rename record has no XY status")?;
                for _ in 0..6 {
                    fields.next().ok_or("rename record is incomplete")?;
                }
                let score = fields.next().ok_or("rename record has no score")?;
                let path = fields.next().ok_or("rename record has no path")?;
                let old_path = entries
                    .get(index + 1)
                    .ok_or("rename record has no original path")?;
                let kind = if score.starts_with('C') {
                    ChangeKind::Copied
                } else {
                    ChangeKind::Renamed
                };
                let (staged, unstaged) = stages(xy);
                files.push(ChangedFile {
                    path: path.to_owned(),
                    old_path: Some((*old_path).to_owned()),
                    change_kind: kind,
                    staged,
                    unstaged,
                    binary: false,
                    outside_project: outside_project(path, project_prefix),
                });
                index += 1;
            }
            "u" => {
                let mut fields = entry.splitn(11, ' ');
                fields.next();
                let xy = fields.next().ok_or("unmerged record has no XY status")?;
                for _ in 0..8 {
                    fields.next().ok_or("unmerged record is incomplete")?;
                }
                let path = fields.next().ok_or("unmerged record has no path")?;
                let (_, unstaged) = stages(xy);
                files.push(ChangedFile {
                    path: path.to_owned(),
                    old_path: None,
                    change_kind: ChangeKind::Unmerged,
                    staged: true,
                    unstaged,
                    binary: false,
                    outside_project: outside_project(path, project_prefix),
                });
            }
            "?" => {
                let path = entry
                    .strip_prefix("? ")
                    .ok_or("untracked record has no path")?;
                files.push(ChangedFile {
                    path: path.to_owned(),
                    old_path: None,
                    change_kind: ChangeKind::Untracked,
                    staged: false,
                    unstaged: true,
                    binary: false,
                    outside_project: outside_project(path, project_prefix),
                });
            }
            "!" => {}
            other => return Err(format!("unknown status record: {other}")),
        }
        index += 1;
    }
    let has_conflicts = files
        .iter()
        .any(|file| file.change_kind == ChangeKind::Unmerged);
    let has_existing_staged_changes = files.iter().any(|file| file.staged);
    Ok(WorktreeSnapshot {
        status_token: status_token(output),
        files,
        has_conflicts,
        has_existing_staged_changes,
    })
}

fn changed(
    path: &str,
    old_path: Option<String>,
    xy: &str,
    project_prefix: Option<&str>,
) -> ChangedFile {
    let (staged, unstaged) = stages(xy);
    let x = xy.as_bytes().first().copied().unwrap_or(b'.');
    let y = xy.as_bytes().get(1).copied().unwrap_or(b'.');
    let code = if x != b'.' && x != b' ' { x } else { y };
    let change_kind = match code {
        b'A' => ChangeKind::Added,
        b'D' => ChangeKind::Deleted,
        b'T' => ChangeKind::TypeChanged,
        b'U' => ChangeKind::Unmerged,
        _ => ChangeKind::Modified,
    };
    ChangedFile {
        path: path.to_owned(),
        old_path,
        change_kind,
        staged,
        unstaged,
        binary: false,
        outside_project: outside_project(path, project_prefix),
    }
}

fn stages(xy: &str) -> (bool, bool) {
    let bytes = xy.as_bytes();
    (
        bytes.first().is_some_and(|c| *c != b'.' && *c != b' '),
        bytes.get(1).is_some_and(|c| *c != b'.' && *c != b' '),
    )
}
fn outside_project(path: &str, project_prefix: Option<&str>) -> bool {
    project_prefix.is_some_and(|prefix| !path.starts_with(prefix))
}

fn status_token(output: &str) -> String {
    // A stable, non-secret checksum sufficient for detecting a changed preview.
    let hash = output
        .as_bytes()
        .iter()
        .fold(0xcbf29ce484222325u64, |value, byte| {
            (value ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
        });
    format!("{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_paths_and_rename_with_nul_separation() {
        let value = concat!(
            "1 .M N... 100644 100644 100644 abc abc Assets/日本語 file.txt\0? Library/new file\0",
            "2 R. N... 100644 100644 100644 abc def R100 Assets/new.txt\0Assets/old.txt\0"
        );
        let snapshot = parse_porcelain_v2(value, Some("Assets/")).unwrap();
        assert_eq!(snapshot.files.len(), 3);
        assert!(snapshot.files[0].unstaged);
        assert!(snapshot.files[1].outside_project);
        assert_eq!(
            snapshot.files[2].old_path.as_deref(),
            Some("Assets/old.txt")
        );
        assert!(snapshot.has_existing_staged_changes);
    }
    #[test]
    fn parses_unmerged_status() {
        let snapshot = parse_porcelain_v2(
            "u UU N... 100644 100644 100644 100644 abc def ghi Assets/a.txt\0",
            None,
        )
        .unwrap();
        assert!(snapshot.has_conflicts);
        assert_eq!(snapshot.files[0].change_kind, ChangeKind::Unmerged);
    }
}
