//! What the agent changed in each repo of the machine, against the commit it was cloned at:
//! the commits since, and the diff of the work tree (uncommitted and new files included, ignored
//! files never) and of each commit. Reported to the server at the end of every turn.

use std::path::Path;

use antilog_types::{
    COMMIT_CAP, ChangesReport, CommitDiff, CommitInfo, DIFF_BYTE_CAP, FileChange, FileStatus,
    RepoReport,
};
use anyhow::{Result, bail};

use super::sandbox::{Exec, Machine, Output};

/// More changed lines than this in one diff and no patch is read: every file is "too large".
const LINE_CAP: u64 = 200_000;

/// Git's empty tree, the parent of a commit that has none.
const EMPTY_TREE: &str = "4b825dc642cb6eb9a060e54bf8d69288fbd7e0e1";

/// Where a repo was when it was cloned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Base {
    pub branch: String,
    pub sha: String,
}

async fn git<M: Machine>(
    machine: &M,
    dir: &Path,
    args: &[&str],
    index: Option<&str>,
) -> Result<Output> {
    let mut words = vec![
        "git",
        "-c",
        "core.quotepath=false",
        "-c",
        "diff.noprefix=false",
    ];
    words.extend_from_slice(args);
    let mut exec = Exec::new(words).dir(dir);
    if let Some(index) = index {
        exec = exec.env("GIT_INDEX_FILE", index);
    }
    machine.exec(exec).await
}

async fn text<M: Machine>(machine: &M, dir: &Path, args: &[&str]) -> Result<String> {
    let out = git(machine, dir, args, None).await?;
    if out.status != 0 {
        bail!(
            "git {} failed: {}",
            args.first().copied().unwrap_or_default(),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_owned())
}

/// The branch and commit the repo is at now: what its changes are measured from.
pub async fn base<M: Machine>(machine: &M, dir: &Path) -> Result<Base> {
    Ok(Base {
        branch: text(machine, dir, &["rev-parse", "--abbrev-ref", "HEAD"]).await?,
        sha: text(machine, dir, &["rev-parse", "HEAD"]).await?,
    })
}

/// One numstat line: lines added and removed (`None` for a binary file) and the path.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Stat {
    path: String,
    added: Option<u32>,
    removed: Option<u32>,
}

/// `git diff --numstat -z` without renames: `<added>\t<removed>\t<path>\0` for each file.
fn parse_numstat(raw: &str) -> Vec<Stat> {
    raw.split('\0')
        .filter(|entry| !entry.is_empty())
        .filter_map(|entry| {
            let mut parts = entry.trim_start_matches('\n').splitn(3, '\t');
            let added = parts.next()?;
            let removed = parts.next()?;
            let path = parts.next()?;
            Some(Stat {
                path: path.to_owned(),
                added: added.parse().ok(),
                removed: removed.parse().ok(),
            })
        })
        .collect()
}

/// `git diff --name-status -z` without renames: `<letter>\0<path>\0` for each file.
fn parse_statuses(raw: &str) -> Vec<(String, FileStatus)> {
    let mut parts = raw.split('\0').filter(|part| !part.is_empty());
    let mut out = Vec::new();
    while let (Some(letter), Some(path)) = (parts.next(), parts.next()) {
        let status = match letter.chars().next() {
            Some('A') => FileStatus::Added,
            Some('D') => FileStatus::Deleted,
            _ => FileStatus::Modified,
        };
        out.push((path.to_owned(), status));
    }
    out
}

/// A unified diff split by file, each from its first `@@` line on. Files come in the order
/// git prints them, which is the order of `--numstat`; a binary file has no text.
fn split_patches(diff: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut open = false;
    for line in diff.split_inclusive('\n') {
        if line.starts_with("diff --git ") {
            out.push(String::new());
            open = false;
        } else if let Some(patch) = out.last_mut() {
            if line.starts_with("@@") {
                open = true;
            }
            if open {
                patch.push_str(line);
            }
        }
    }
    out
}

/// Files with their patches, within `DIFF_BYTE_CAP` bytes of patch text. A file that does not
/// fit keeps its counts and loses its patch.
fn assemble(
    stats: Vec<Stat>,
    statuses: &[(String, FileStatus)],
    patches: Option<Vec<String>>,
) -> Vec<FileChange> {
    let mut budget = DIFF_BYTE_CAP;
    let aligned = patches.as_ref().is_some_and(|all| all.len() == stats.len());
    stats
        .into_iter()
        .enumerate()
        .map(|(n, stat)| {
            let binary = stat.added.is_none() || stat.removed.is_none();
            let status = statuses
                .iter()
                .find(|(path, _)| *path == stat.path)
                .map_or(FileStatus::Modified, |(_, status)| *status);
            let mut patch = if binary || !aligned {
                String::new()
            } else {
                patches
                    .as_ref()
                    .and_then(|all| all.get(n))
                    .cloned()
                    .unwrap_or_default()
            };
            let mut too_large = !binary && !aligned;
            if patch.len() > budget {
                patch.clear();
                too_large = true;
            }
            budget -= patch.len();
            FileChange {
                path: stat.path,
                old_path: None,
                status,
                additions: stat.added.unwrap_or(0),
                deletions: stat.removed.unwrap_or(0),
                binary,
                too_large,
                patch,
            }
        })
        .collect()
}

/// `git diff` with `extra` options over `range`, as text.
async fn diff_text<M: Machine>(
    machine: &M,
    dir: &Path,
    extra: &[&str],
    range: &[&str],
    index: Option<&str>,
) -> Result<String> {
    let mut args = vec!["diff", "--no-color", "--no-ext-diff", "--no-renames", "-U3"];
    args.extend_from_slice(extra);
    args.extend_from_slice(range);
    let out = git(machine, dir, &args, index).await?;
    if out.status != 0 {
        bail!(
            "git diff failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// The diff of `range` (one or two revisions), through `index` when given.
async fn diff<M: Machine>(
    machine: &M,
    dir: &Path,
    range: &[&str],
    index: Option<&str>,
) -> Result<Vec<FileChange>> {
    let stats = parse_numstat(&diff_text(machine, dir, &["--numstat", "-z"], range, index).await?);
    if stats.is_empty() {
        return Ok(Vec::new());
    }
    let statuses =
        parse_statuses(&diff_text(machine, dir, &["--name-status", "-z"], range, index).await?);
    let lines: u64 = stats
        .iter()
        .map(|stat| u64::from(stat.added.unwrap_or(0)) + u64::from(stat.removed.unwrap_or(0)))
        .sum();
    let patches = if lines > LINE_CAP {
        None
    } else {
        Some(split_patches(
            &diff_text(machine, dir, &[], range, index).await?,
        ))
    };
    Ok(assemble(stats, &statuses, patches))
}

fn parse_commits(raw: &str) -> Vec<CommitInfo> {
    raw.split('\x1e')
        .filter_map(|entry| {
            let mut parts = entry.trim().splitn(3, '\x1f');
            let sha = parts.next().filter(|sha| !sha.is_empty())?;
            let subject = parts.next()?;
            let seconds: i64 = parts.next()?.trim().parse().ok()?;
            Some(CommitInfo {
                sha: sha.to_owned(),
                subject: subject.to_owned(),
                time: seconds * 1000,
            })
        })
        .collect()
}

/// One repo's report: `name` is how the session names it, `dir` where it is cloned.
pub async fn repo<M: Machine>(
    machine: &M,
    name: &str,
    dir: &Path,
    base: &Base,
) -> Result<RepoReport> {
    let head_sha = text(machine, dir, &["rev-parse", "HEAD"]).await?;
    let head_branch = text(machine, dir, &["rev-parse", "--abbrev-ref", "HEAD"]).await?;
    let range = format!("{}..HEAD", base.sha);
    let limit = format!("-n{COMMIT_CAP}");
    let commits = parse_commits(
        &text(
            machine,
            dir,
            &["log", "--format=%H%x1f%s%x1f%ct%x1e", &limit, &range],
        )
        .await?,
    );

    // New files count as changes, and the real index stays as the agent left it: the diff runs
    // through a copy of it with the untracked, not ignored, files added as intent.
    let git_dir = text(machine, dir, &["rev-parse", "--absolute-git-dir"]).await?;
    let copy = format!("{git_dir}/antilog-index");
    machine
        .exec(Exec::new([
            "cp".to_owned(),
            "-f".to_owned(),
            format!("{git_dir}/index"),
            copy.clone(),
        ]))
        .await?;
    git(
        machine,
        dir,
        &["add", "--intent-to-add", "--all", "."],
        Some(&copy),
    )
    .await?;
    let files = diff(machine, dir, &[base.sha.as_str()], Some(&copy)).await;
    machine
        .exec(Exec::new(["rm".to_owned(), "-f".to_owned(), copy]))
        .await
        .ok();
    let files = files?;

    let mut commit_diffs = Vec::new();
    let mut budget = DIFF_BYTE_CAP;
    for commit in &commits {
        let parent = text(machine, dir, &["rev-list", "--parents", "-n1", &commit.sha])
            .await?
            .split_whitespace()
            .nth(1)
            .map_or_else(|| EMPTY_TREE.to_owned(), str::to_owned);
        let mut files = diff(machine, dir, &[parent.as_str(), commit.sha.as_str()], None).await?;
        for file in &mut files {
            if file.patch.len() > budget {
                file.patch.clear();
                file.too_large = !file.binary;
            }
            budget -= file.patch.len();
        }
        commit_diffs.push(CommitDiff {
            sha: commit.sha.clone(),
            files,
        });
    }
    Ok(RepoReport {
        repo: name.to_owned(),
        base_branch: base.branch.clone(),
        head_branch,
        base_sha: base.sha.clone(),
        head_sha,
        commits,
        files,
        commit_diffs,
    })
}

/// The report for every repo that has a recorded base. A repo that cannot be read is left out
/// and logged: the next turn tries again.
pub async fn report<M: Machine>(
    machine: &M,
    repos: &[(String, std::path::PathBuf, Base)],
) -> ChangesReport {
    let mut out = Vec::new();
    for (name, dir, base) in repos {
        match repo(machine, name, dir, base).await {
            Ok(report) => out.push(report),
            Err(error) => tracing::warn!(repo = %name, %error, "could not read the repo's changes"),
        }
    }
    ChangesReport { repos: out }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numstat_reads_counts_binary_and_paths_with_spaces() {
        let raw = "3\t1\tsrc/a b.rs\0-\t-\timg.png\0";
        assert_eq!(
            parse_numstat(raw),
            vec![
                Stat {
                    path: "src/a b.rs".into(),
                    added: Some(3),
                    removed: Some(1)
                },
                Stat {
                    path: "img.png".into(),
                    added: None,
                    removed: None
                },
            ]
        );
    }

    #[test]
    fn a_diff_is_split_by_file_from_its_first_hunk() {
        let diff = "diff --git a/x b/x\nindex 1..2 100644\n--- a/x\n+++ b/x\n@@ -1 +1 @@\n-a\n+b\ndiff --git a/i.png b/i.png\nBinary files a/i.png and b/i.png differ\ndiff --git a/y b/y\n--- a/y\n+++ b/y\n@@ -0,0 +1 @@\n+c\n";
        assert_eq!(
            split_patches(diff),
            vec![
                "@@ -1 +1 @@\n-a\n+b\n".to_owned(),
                String::new(),
                "@@ -0,0 +1 @@\n+c\n".to_owned()
            ]
        );
    }

    #[test]
    fn a_file_past_the_byte_cap_keeps_its_counts_and_loses_its_patch() {
        let stats = vec![
            Stat {
                path: "big".into(),
                added: Some(9),
                removed: Some(0),
            },
            Stat {
                path: "img".into(),
                added: None,
                removed: None,
            },
            Stat {
                path: "small".into(),
                added: Some(1),
                removed: Some(0),
            },
        ];
        let patches = vec![
            "+".repeat(DIFF_BYTE_CAP + 1),
            String::new(),
            "@@\n+x\n".into(),
        ];
        let files = assemble(stats, &[("big".into(), FileStatus::Added)], Some(patches));
        assert!(files[0].too_large && files[0].patch.is_empty() && files[0].additions == 9);
        assert_eq!(files[0].status, FileStatus::Added);
        assert!(files[1].binary && !files[1].too_large);
        assert_eq!(files[2].patch, "@@\n+x\n");
    }

    #[test]
    fn without_patches_every_text_file_is_too_large() {
        let stats = vec![Stat {
            path: "a".into(),
            added: Some(1),
            removed: Some(1),
        }];
        assert!(assemble(stats, &[], None)[0].too_large);
    }

    #[test]
    fn commits_are_read_with_subject_and_time() {
        let raw = "abc\x1ffix: a thing\x1f1700000000\x1e\ndef\x1fsecond\x1f1700000100\x1e";
        let commits = parse_commits(raw);
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].subject, "fix: a thing");
        assert_eq!(commits[0].time, 1_700_000_000_000);
    }
}
