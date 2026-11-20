//! The runner's side of `SessionMachine`: check out the commits, restore the protected files,
//! run the judge's command and return its exit status and output. An evaluation run happens in
//! the session's machine, through `Machine::exec`. A holdout run happens in a second machine
//! started for that run alone, from the same environment, and removed when it ends: the agent's
//! machine never sees it.

use std::path::{Path, PathBuf};

use antilog_types::judge::ProtectedFile;
use antilog_types::{RunSpec, SessionSpec};
use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use super::sandbox::{Exec, Machine, Sandbox, SandboxSpec};
use super::session::clone_all;

/// What the command printed, last bytes kept.
const OUTPUT_LIMIT: usize = 1 << 20;

/// Where a repo of the session is cloned.
pub fn repo_dir(workdir: &Path, repo: &str) -> PathBuf {
    workdir.join(repo.rsplit('/').next().unwrap_or(repo))
}

async fn git<M: Machine>(machine: &M, dir: &Path, args: &[&str]) -> Result<Vec<u8>> {
    let mut command = vec![
        "git".to_owned(),
        "-C".to_owned(),
        dir.to_string_lossy().into_owned(),
    ];
    command.extend(args.iter().map(|arg| (*arg).to_owned()));
    let output = machine
        .exec(Exec::new(command))
        .await
        .context("running git")?;
    if output.status != 0 {
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
async fn restore<M: Machine>(
    machine: &M,
    repo: &Path,
    checkout: &Path,
    file: &ProtectedFile,
) -> Result<()> {
    let want = file.sha256.trim_start_matches("sha256:");
    let target = checkout.join(&file.path);
    let current = machine
        .exec(Exec::new([
            "cat".to_owned(),
            target.to_string_lossy().into_owned(),
        ]))
        .await?;
    if current.status == 0 && digest(&current.stdout) == want {
        return Ok(());
    }
    let history = git(
        machine,
        repo,
        &["log", "--all", "--format=%H", "--", &file.path],
    )
    .await?;
    for commit in String::from_utf8_lossy(&history).lines() {
        let Ok(content) = git(machine, repo, &["show", &format!("{commit}:{}", file.path)]).await
        else {
            continue;
        };
        if digest(&content) == want {
            machine.put(&target, &content).await?;
            return Ok(());
        }
    }
    bail!(
        "the sealed {} of {} is not in the repo's history",
        file.path,
        file.repo
    )
}

async fn prepare<M: Machine>(
    machine: &M,
    workdir: &Path,
    root: &Path,
    spec: &RunSpec,
) -> Result<Vec<(PathBuf, PathBuf)>> {
    let mut trees = Vec::new();
    for commit in &spec.commits {
        let repo = repo_dir(workdir, &commit.repo);
        if git(
            machine,
            &repo,
            &["cat-file", "-e", &format!("{}^{{commit}}", commit.sha)],
        )
        .await
        .is_err()
        {
            // Not in the clone yet: the trial may have been pushed from elsewhere.
            git(machine, &repo, &["fetch", "--all", "--quiet"])
                .await
                .ok();
        }
        let checkout = repo_dir(root, &commit.repo);
        let target = checkout.to_string_lossy().into_owned();
        git(
            machine,
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
        restore(machine, repo, checkout, file).await?;
    }
    Ok(trees)
}

/// Runs one judge command. Evaluation data runs in the session's machine. Holdout data runs in
/// a fresh machine of the session's environment, with the repos cloned from the code host (not
/// copied from the agent's machine, so nothing the agent left there decides the run), and the
/// machine is removed whatever the outcome.
pub async fn run<S: Sandbox>(
    sandbox: &S,
    live: &S::Machine,
    session: &SessionSpec,
    id: Uuid,
    spec: &RunSpec,
) -> antilog_types::RunResult {
    if !spec.data.holdout {
        return run_in(live, id, spec).await;
    }
    let fresh = match sandbox.start(&SandboxSpec::new(session).for_run(id)).await {
        Ok(fresh) => fresh,
        Err(error) => return failed(id, &error.context("starting the holdout machine")),
    };
    let result = match clone_all(&fresh, &fresh.paths().work, session).await {
        Ok(()) => run_in(&fresh, id, spec).await,
        Err(error) => failed(id, &error.context("cloning the repos for the holdout run")),
    };
    fresh.stop().await;
    result
}

fn failed(id: Uuid, error: &anyhow::Error) -> antilog_types::RunResult {
    antilog_types::RunResult {
        command: id,
        exit: 1,
        output: format!("{error:#}"),
    }
}

/// Runs the command on the commits in one machine. A checkout or restore that fails is a run
/// that failed, with the reason as its output, so the judge never reads a number from a run that
/// was not clean.
async fn run_in<M: Machine>(machine: &M, id: Uuid, spec: &RunSpec) -> antilog_types::RunResult {
    let workdir = machine.paths().work.clone();
    let root = workdir.join(".antilog-run").join(id.to_string());
    let mut trees = Vec::new();
    let outcome = async {
        machine
            .exec(Exec::new([
                "mkdir".to_owned(),
                "-p".to_owned(),
                root.to_string_lossy().into_owned(),
            ]))
            .await?
            .ok("making the run's folder")?;
        trees = prepare(machine, &workdir, &root, spec).await?;
        let cwd = match trees.as_slice() {
            [(_, only)] => only.clone(),
            _ => root.clone(),
        };
        let script = format!("{{ {}\n}} 2>&1", spec.command);
        let output = machine
            .exec(
                Exec::new(["sh".to_owned(), "-c".to_owned(), script])
                    .dir(&cwd)
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
                        &spec.data.dataset.version.to_string(),
                    )
                    .env("ANTILOG_REPOS_ROOT", &root.to_string_lossy()),
            )
            .await
            .context("running the command")?;
        let mut text = output.stdout;
        if text.len() > OUTPUT_LIMIT {
            text.drain(..text.len() - OUTPUT_LIMIT);
        }
        anyhow::Ok((output.status, String::from_utf8_lossy(&text).into_owned()))
    }
    .await;
    for (repo, checkout) in &trees {
        let target = checkout.to_string_lossy().into_owned();
        git(machine, repo, &["worktree", "remove", "--force", &target])
            .await
            .ok();
    }
    machine
        .exec(Exec::new([
            "rm".to_owned(),
            "-rf".to_owned(),
            root.to_string_lossy().into_owned(),
        ]))
        .await
        .ok();
    match outcome {
        Ok((exit, output)) => antilog_types::RunResult {
            command: id,
            exit,
            output,
        },
        Err(error) => failed(id, &error),
    }
}

#[cfg(test)]
mod tests {
    use antilog_types::DataSpec;
    use antilog_types::judge::{CommitRef, DatasetRef};

    use super::super::sandbox::Paths;
    use super::super::sandbox::plain::{Plain, PlainMachine};
    use std::sync::{Arc, Mutex};
    use tokio::process::Command;

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
        let machine = PlainMachine::new(
            Paths {
                work: temp.clone(),
                config: temp.join("opencode.json"),
                skills: temp.join("skills"),
            },
            Vec::new(),
        );
        let result = run_in(
            &machine,
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
        let result = run_in(
            &machine,
            Uuid::now_v7(),
            &spec("echo hi", &sha, vec![forged]),
        )
        .await;
        assert_eq!(result.exit, 1);
        assert!(
            result.output.contains("not in the repo's history"),
            "{}",
            result.output
        );
        tokio::fs::remove_dir_all(&temp).await.ok();
    }

    /// A `Plain` sandbox that writes what happens to each machine into one log.
    struct Logged {
        plain: Plain,
        log: Arc<Mutex<Vec<String>>>,
    }

    struct LoggedMachine {
        inner: PlainMachine,
        name: String,
        log: Arc<Mutex<Vec<String>>>,
    }

    impl Sandbox for Logged {
        type Machine = LoggedMachine;

        async fn start(&self, spec: &SandboxSpec) -> Result<LoggedMachine> {
            self.log
                .lock()
                .unwrap()
                .push(format!("start {}", spec.name()));
            Ok(LoggedMachine {
                inner: self.plain.start(spec).await?,
                name: spec.name(),
                log: self.log.clone(),
            })
        }
    }

    impl Machine for LoggedMachine {
        fn paths(&self) -> &Paths {
            self.inner.paths()
        }

        async fn exec(&self, exec: Exec) -> Result<super::super::sandbox::Output> {
            self.log
                .lock()
                .unwrap()
                .push(format!("exec {} {}", self.name, exec.args.join(" ")));
            self.inner.exec(exec).await
        }

        async fn spawn(&self, exec: Exec) -> Result<()> {
            self.inner.spawn(exec).await
        }

        async fn listener(&self) -> Result<super::super::sandbox::Listener> {
            self.inner.listener().await
        }

        async fn stop(&self) {
            self.log.lock().unwrap().push(format!("stop {}", self.name));
            self.inner.stop().await;
        }
    }

    fn session(repo_url: &str) -> SessionSpec {
        SessionSpec {
            session: antilog_types::SessionId::new(),
            organization: antilog_types::OrganizationId::new(),
            experiment: antilog_types::ExperimentId::new(),
            agent: "opencode".into(),
            model: "v/m".into(),
            repos: vec![antilog_types::RepoSpec {
                name: "acme/app".into(),
                url: repo_url.into(),
                token: None,
            }],
            environment: antilog_types::EnvironmentSpec::default(),
            skills: Vec::new(),
            opencode_config: serde_json::json!({}),
            model_token: String::new(),
            mcp_token: String::new(),
            runner_token: String::new(),
            resume: false,
        }
    }

    #[tokio::test]
    async fn a_holdout_run_uses_a_second_machine_that_is_removed_and_the_live_one_sees_nothing() {
        let temp = std::env::temp_dir().join(format!("antilog-holdout-{}", Uuid::now_v7()));
        let origin = temp.join("origin");
        tokio::fs::create_dir_all(&origin).await.unwrap();
        sh(
            &origin,
            "git init -q . && git config user.email a@b && git config user.name n && echo v1 > code.txt && git add . && git commit -qm base",
        )
        .await;
        let sha = sh(&origin, "git rev-parse HEAD").await;
        let log = Arc::new(Mutex::new(Vec::new()));
        let sandbox = Logged {
            plain: Plain::under(&temp),
            log: log.clone(),
        };
        let session = session(&origin.to_string_lossy());
        let live = sandbox.start(&SandboxSpec::new(&session)).await.unwrap();
        let live_name = session.session.to_string();
        log.lock().unwrap().clear();

        // An evaluation run stays in the live machine: no second machine.
        let mut evaluation = spec("true", &sha, Vec::new());
        evaluation.data.holdout = false;
        let id = Uuid::now_v7();
        run(&sandbox, &live, &session, id, &evaluation).await;
        {
            let log = log.lock().unwrap();
            assert!(!log.iter().any(|l| l.starts_with("start ")), "{log:?}");
            assert!(
                log.iter()
                    .any(|l| l.starts_with(&format!("exec {live_name} ")))
            );
        }
        log.lock().unwrap().clear();

        // A holdout run starts a machine for the run, runs there and removes it.
        let id = Uuid::now_v7();
        let result = run(
            &sandbox,
            &live,
            &session,
            id,
            &spec("cat code.txt; echo $ANTILOG_DATA_KIND", &sha, Vec::new()),
        )
        .await;
        assert_eq!((result.exit, result.output.as_str()), (0, "v1\nholdout\n"));
        let fresh = format!("{live_name}-{id}");
        let log = log.lock().unwrap().clone();
        assert_eq!(log.first().unwrap(), &format!("start {fresh}"), "{log:?}");
        assert_eq!(log.last().unwrap(), &format!("stop {fresh}"), "{log:?}");
        assert_eq!(
            log.iter().filter(|l| l.starts_with("stop ")).count(),
            1,
            "{log:?}"
        );
        assert!(
            log.iter()
                .all(|l| l.starts_with("start ") || l.starts_with("stop ") || l.contains(&fresh)),
            "the live machine saw an exec: {log:?}"
        );
        assert!(
            !log.iter()
                .any(|l| l.contains(&format!("exec {live_name} ")))
        );

        // A run that fails in the machine still removes it.
        log_clear(&sandbox);
        let id = Uuid::now_v7();
        let missing = "0".repeat(40);
        let result = run(
            &sandbox,
            &live,
            &session,
            id,
            &spec("true", &missing, Vec::new()),
        )
        .await;
        assert_eq!(result.exit, 1);
        let log = sandbox.log.lock().unwrap().clone();
        assert_eq!(
            log.last().unwrap(),
            &format!("stop {live_name}-{id}"),
            "{log:?}"
        );

        // So does a clone that fails.
        log_clear(&sandbox);
        let broken = SessionSpec {
            repos: vec![antilog_types::RepoSpec {
                name: "acme/app".into(),
                url: temp.join("nowhere").to_string_lossy().into_owned(),
                token: None,
            }],
            ..session.clone()
        };
        let id = Uuid::now_v7();
        let result = run(
            &sandbox,
            &live,
            &broken,
            id,
            &spec("true", &sha, Vec::new()),
        )
        .await;
        assert_eq!(result.exit, 1);
        assert!(
            result.output.contains("cloning the repos"),
            "{}",
            result.output
        );
        let log = sandbox.log.lock().unwrap().clone();
        assert_eq!(
            log.last().unwrap(),
            &format!("stop {live_name}-{id}"),
            "{log:?}"
        );
        tokio::fs::remove_dir_all(&temp).await.ok();
    }

    fn log_clear(sandbox: &Logged) {
        sandbox.log.lock().unwrap().clear();
    }
}
