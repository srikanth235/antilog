//! The runner's side of `SessionMachine`: check out the commits, restore the protected files,
//! run the judge's command and return its exit status and output.

use std::path::{Path, PathBuf};

use antilog_types::RunSpec;
use antilog_types::judge::ProtectedFile;
use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};
use tokio::process::Command;
use uuid::Uuid;

/// What the command printed, last bytes kept.
const OUTPUT_LIMIT: usize = 1 << 20;

/// Where a repo of the session is cloned.
pub fn repo_dir(workdir: &Path, repo: &str) -> PathBuf {
    workdir.join(repo.rsplit('/').next().unwrap_or(repo))
}

async fn git(dir: &Path, args: &[&str]) -> Result<Vec<u8>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .await
        .context("running git")?;
    if !output.status.success() {
        bail!(
            "git {} failed: {}",
            args.first().copied().unwrap_or_default(),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(output.stdout)
}

fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// Puts the sealed content of a protected file back. The seal names a file by its hash, so the
/// content is looked for, by that hash, in the repo's own history: never taken from the trial.
async fn restore(repo: &Path, checkout: &Path, file: &ProtectedFile) -> Result<()> {
    let want = file.sha256.trim_start_matches("sha256:");
    let target = checkout.join(&file.path);
    if let Ok(current) = tokio::fs::read(&target).await
        && digest(&current) == want
    {
        return Ok(());
    }
    let history = git(repo, &["log", "--all", "--format=%H", "--", &file.path]).await?;
    for commit in String::from_utf8_lossy(&history).lines() {
        let Ok(content) = git(repo, &["show", &format!("{commit}:{}", file.path)]).await else {
            continue;
        };
        if digest(&content) == want {
            if let Some(parent) = target.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }
            tokio::fs::write(&target, content).await?;
            return Ok(());
        }
    }
    bail!(
        "the sealed {} of {} is not in the repo's history",
        file.path,
        file.repo
    )
}

async fn prepare(workdir: &Path, root: &Path, spec: &RunSpec) -> Result<Vec<(PathBuf, PathBuf)>> {
    let mut trees = Vec::new();
    for commit in &spec.commits {
        let repo = repo_dir(workdir, &commit.repo);
        if git(
            &repo,
            &["cat-file", "-e", &format!("{}^{{commit}}", commit.sha)],
        )
        .await
        .is_err()
        {
            // Not in the clone yet: the trial may have been pushed from elsewhere.
            git(&repo, &["fetch", "--all", "--quiet"]).await.ok();
        }
        let checkout = repo_dir(root, &commit.repo);
        let target = checkout.to_string_lossy().into_owned();
        git(
            &repo,
            &["worktree", "add", "--detach", &target, &commit.sha],
        )
        .await?;
        trees.push((repo, checkout));
    }
    for file in &spec.files {
        let (repo, checkout) = trees
            .iter()
            .find(|(repo, _)| *repo == repo_dir(workdir, &file.repo))
            .with_context(|| {
                format!("protected file {} names a repo not checked out", file.path)
            })?;
        restore(repo, checkout, file).await?;
    }
    Ok(trees)
}

/// Runs the command on the commits. A checkout or restore that fails is a run that failed, with
/// the reason as its output, so the judge never reads a number from a run that was not clean.
pub async fn run(workdir: &Path, id: Uuid, spec: &RunSpec) -> antilog_types::RunResult {
    let root = workdir.join(".antilog-run").join(id.to_string());
    let mut trees = Vec::new();
    let outcome = async {
        tokio::fs::create_dir_all(&root).await?;
        trees = prepare(workdir, &root, spec).await?;
        let cwd = match trees.as_slice() {
            [(_, only)] => only.clone(),
            _ => root.clone(),
        };
        let script = format!("{{ {}\n}} 2>&1", spec.command);
        let output = Command::new("sh")
            .arg("-c")
            .arg(script)
            .current_dir(cwd)
            .env_remove("ANTILOG_RUNNER_KEY")
            .env(
                "ANTILOG_DATA_KIND",
                if spec.data.holdout {
                    "holdout"
                } else {
                    "evaluation"
                },
            )
            .env("ANTILOG_DATASET_ID", &spec.data.dataset.id)
            .env(
                "ANTILOG_DATASET_VERSION",
                spec.data.dataset.version.to_string(),
            )
            .env("ANTILOG_REPOS_ROOT", &root)
            .kill_on_drop(true)
            .output()
            .await
            .context("running the command")?;
        let mut text = output.stdout;
        if text.len() > OUTPUT_LIMIT {
            text.drain(..text.len() - OUTPUT_LIMIT);
        }
        anyhow::Ok((
            output.status.code().unwrap_or(1),
            String::from_utf8_lossy(&text).into_owned(),
        ))
    }
    .await;
    for (repo, checkout) in &trees {
        let target = checkout.to_string_lossy().into_owned();
        git(repo, &["worktree", "remove", "--force", &target])
            .await
            .ok();
    }
    tokio::fs::remove_dir_all(&root).await.ok();
    match outcome {
        Ok((exit, output)) => antilog_types::RunResult {
            command: id,
            exit,
            output,
        },
        Err(error) => antilog_types::RunResult {
            command: id,
            exit: 1,
            output: format!("{error:#}"),
        },
    }
}

#[cfg(test)]
mod tests {
    use antilog_types::DataSpec;
    use antilog_types::judge::{CommitRef, DatasetRef};

    use super::*;

    async fn sh(dir: &Path, script: &str) -> String {
        let output = Command::new("sh")
            .arg("-c")
            .arg(script)
            .current_dir(dir)
            .output()
            .await
            .unwrap();
        assert!(
            output.status.success(),
            "{script}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).trim().to_owned()
    }

    fn spec(command: &str, sha: &str, files: Vec<ProtectedFile>) -> RunSpec {
        RunSpec {
            command: command.to_owned(),
            commits: vec![CommitRef {
                repo: "acme/app".into(),
                sha: sha.into(),
            }],
            files,
            data: DataSpec {
                holdout: true,
                dataset: DatasetRef {
                    id: "d1".into(),
                    version: 2,
                },
            },
        }
    }

    #[tokio::test]
    async fn the_command_runs_on_the_commit_with_the_sealed_files_put_back() {
        let temp = std::env::temp_dir().join(format!("antilog-machine-{}", Uuid::now_v7()));
        let repo = temp.join("app");
        tokio::fs::create_dir_all(&repo).await.unwrap();
        sh(
            &repo,
            "git init -q . && git config user.email a@b && git config user.name n",
        )
        .await;
        sh(&repo, "mkdir bench && echo 'echo honest' > bench/run.sh && echo v1 > code.txt && git add . && git commit -qm base").await;
        let sealed = digest(&tokio::fs::read(repo.join("bench/run.sh")).await.unwrap());
        // The trial changes the evaluation to cheat, and the code.
        sh(
            &repo,
            "echo 'echo cheat' > bench/run.sh && echo v2 > code.txt && git commit -qam trial",
        )
        .await;
        let sha = sh(&repo, "git rev-parse HEAD").await;
        // The clone then moves on: the run must not use the working tree.
        sh(&repo, "git checkout -q -b other HEAD~1").await;
        let file = ProtectedFile {
            repo: "acme/app".into(),
            path: "bench/run.sh".into(),
            sha256: sealed,
        };
        let id = Uuid::now_v7();
        let result = run(
            &temp,
            id,
            &spec("cat code.txt; sh bench/run.sh; echo \"$ANTILOG_DATA_KIND $ANTILOG_DATASET_ID@$ANTILOG_DATASET_VERSION\"; exit 3", &sha, vec![file.clone()]),
        )
        .await;
        assert_eq!(result.command, id);
        assert_eq!(result.exit, 3);
        assert_eq!(result.output, "v2\nhonest\nholdout d1@2\n");
        // Nothing is left behind, and the clone's own state is untouched.
        assert!(!temp.join(".antilog-run").join(id.to_string()).exists());
        assert_eq!(sh(&repo, "git branch --show-current").await, "other");

        // A hash that is nowhere in the history is a run that failed, never a run on the trial's file.
        let forged = ProtectedFile {
            sha256: digest(b"nothing"),
            ..file
        };
        let result = run(&temp, Uuid::now_v7(), &spec("echo hi", &sha, vec![forged])).await;
        assert_eq!(result.exit, 1);
        assert!(
            result.output.contains("not in the repo's history"),
            "{}",
            result.output
        );
        tokio::fs::remove_dir_all(&temp).await.ok();
    }
}
