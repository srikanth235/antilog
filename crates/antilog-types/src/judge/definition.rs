use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use utoipa::ToSchema;

use super::{CommitRef, ValueModelId, canonical_json, hex};

pub use crate::ExperimentId;

pub const CRITERIA_VERSION: u32 = 1;
pub const ALPHA: f64 = 0.05;

pub const STANDARD_CHEATS: [&str; 4] = [
    "skip the work",
    "hard-code the answer",
    "return early",
    "behave differently on the holdout",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum Direction {
    Minimize,
    Maximize,
}

impl Direction {
    /// Positive when `trial` is better than `base`.
    pub fn gain(self, base: f64, trial: f64) -> f64 {
        match self {
            Self::Minimize => base - trial,
            Self::Maximize => trial - base,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum EvaluationKind {
    Speed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Metric {
    pub name: String,
    pub unit: String,
    pub direction: Direction,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RepoPin {
    pub repo: String,
    pub commit: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProtectedPath {
    pub repo: String,
    pub glob: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Evaluation {
    pub command: String,
    pub repos: Vec<RepoPin>,
    pub protected: Vec<ProtectedPath>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Guardrail {
    pub name: String,
    pub direction: Direction,
    /// How much worse than its parent the guardrail may get, in its own unit.
    pub tolerance: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DatasetRef {
    pub id: String,
    pub version: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DataRefs {
    pub evaluation: DatasetRef,
    pub holdout: DatasetRef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ValueModelRef {
    pub id: ValueModelId,
    pub version: u32,
}

/// What a person approves. It has no seal until `SealedExperiment::seal`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Definition {
    pub metric: Metric,
    pub kind: EvaluationKind,
    pub evaluation: Evaluation,
    pub guardrails: Vec<Guardrail>,
    pub data: DataRefs,
    pub repeats: u32,
    pub quick_check: bool,
    pub patience: u32,
    pub holdout_cap: u32,
    pub value_model: Option<ValueModelRef>,
    pub criteria: u32,
}

impl Definition {
    /// Protected `repo:path` entries among the changed `(repo, path)` pairs.
    pub fn touched(&self, changed: &[(String, String)]) -> Vec<String> {
        changed
            .iter()
            .filter(|(repo, path)| {
                self.evaluation
                    .protected
                    .iter()
                    .any(|p| p.repo == *repo && glob_matches(&p.glob, path))
            })
            .map(|(repo, path)| format!("{repo}:{path}"))
            .collect()
    }

    pub fn knows_repos(&self, commits: &[CommitRef]) -> bool {
        commits
            .iter()
            .all(|c| self.evaluation.repos.iter().any(|r| r.repo == c.repo))
    }

    pub fn problems(&self) -> Vec<String> {
        let mut out = Vec::new();
        let mut need = |ok: bool, text: &str| {
            if !ok {
                out.push(text.to_owned());
            }
        };
        need(self.repeats >= 2, "repeats must be at least 2");
        need(self.patience >= 1, "patience must be at least 1");
        need(self.holdout_cap >= 1, "the holdout cap must be at least 1");
        need(
            self.criteria == CRITERIA_VERSION,
            "unknown acceptance criteria version",
        );
        need(
            !self.evaluation.command.is_empty(),
            "the evaluation needs a command",
        );
        need(
            !self.evaluation.protected.is_empty(),
            "the evaluation needs protected paths",
        );
        need(
            self.evaluation.repos.iter().all(|r| !r.commit.is_empty()),
            "every repo is pinned to a commit",
        );
        need(
            self.evaluation
                .protected
                .iter()
                .all(|p| self.evaluation.repos.iter().any(|r| r.repo == p.repo)),
            "a protected path names a repo that is not in the evaluation",
        );
        need(
            self.guardrails
                .iter()
                .all(|g| g.tolerance.is_finite() && g.tolerance >= 0.0),
            "a guardrail tolerance is a finite number, zero or more",
        );
        let names: BTreeSet<_> = self.guardrails.iter().map(|g| &g.name).collect();
        need(
            names.len() == self.guardrails.len(),
            "guardrail names are unique",
        );
        need(
            self.data.evaluation != self.data.holdout,
            "the holdout is not the evaluation data",
        );
        out
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProtectedFile {
    pub repo: String,
    pub path: String,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, ToSchema)]
#[serde(transparent)]
pub struct SealHash(String);

impl SealHash {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum SealRefused {
    #[error("the definition is not sealable: {}", .0.join("; "))]
    Definition(Vec<String>),
    #[error("a seal check measured a number that is not finite")]
    NotFinite,
    #[error("the noise is larger than the baseline it would have to beat")]
    TooNoisy,
    #[error("the planted cheat {0:?} was not run")]
    CheatMissing(String),
    #[error("the planted cheat {0:?} was not rejected")]
    CheatNotRejected(String),
    #[error("the seal does not match the definition")]
    Mismatch,
    #[error("the definition cannot be hashed")]
    Unhashable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CheatCheck {
    pub name: String,
    pub rejected: bool,
}

/// Seal check results that passed. It cannot be built from a failing set.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(try_from = "SealChecksWire", rename_all = "camelCase")]
pub struct SealChecks {
    baseline: f64,
    noise: f64,
    cheats: Vec<CheatCheck>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SealChecksWire {
    baseline: f64,
    noise: f64,
    cheats: Vec<CheatCheck>,
}

impl TryFrom<SealChecksWire> for SealChecks {
    type Error = SealRefused;

    fn try_from(w: SealChecksWire) -> Result<Self, SealRefused> {
        Self::passed(w.baseline, w.noise, w.cheats)
    }
}

impl SealChecks {
    /// `noise` is the 95% half-width of one paired difference of the baseline with itself.
    pub fn passed(baseline: f64, noise: f64, cheats: Vec<CheatCheck>) -> Result<Self, SealRefused> {
        if !(baseline.is_finite() && noise.is_finite() && noise >= 0.0) {
            return Err(SealRefused::NotFinite);
        }
        if noise > baseline.abs() {
            return Err(SealRefused::TooNoisy);
        }
        for name in STANDARD_CHEATS {
            if !cheats.iter().any(|c| c.name == name) {
                return Err(SealRefused::CheatMissing(name.to_owned()));
            }
        }
        if let Some(c) = cheats.iter().find(|c| !c.rejected) {
            return Err(SealRefused::CheatNotRejected(c.name.clone()));
        }
        Ok(Self {
            baseline,
            noise,
            cheats,
        })
    }

    pub fn baseline(&self) -> f64 {
        self.baseline
    }

    pub fn noise(&self) -> f64 {
        self.noise
    }

    pub fn cheats(&self) -> &[CheatCheck] {
        &self.cheats
    }
}

/// One immutable experiment version. Only `seal` and a verified load make one.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(try_from = "SealedWire", rename_all = "camelCase")]
pub struct SealedExperiment {
    experiment: ExperimentId,
    version: u32,
    definition: Definition,
    files: Vec<ProtectedFile>,
    checks: SealChecks,
    seal: SealHash,
    /// Epoch milliseconds.
    sealed_at: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SealedWire {
    experiment: ExperimentId,
    version: u32,
    definition: Definition,
    files: Vec<ProtectedFile>,
    checks: SealChecks,
    seal: SealHash,
    sealed_at: i64,
}

impl TryFrom<SealedWire> for SealedExperiment {
    type Error = SealRefused;

    fn try_from(w: SealedWire) -> Result<Self, SealRefused> {
        let sealed = Self::seal(
            w.experiment,
            w.version,
            w.definition,
            w.files,
            w.checks,
            w.sealed_at,
        )?;
        if sealed.seal == w.seal {
            Ok(sealed)
        } else {
            Err(SealRefused::Mismatch)
        }
    }
}

impl SealedExperiment {
    pub fn seal(
        experiment: ExperimentId,
        version: u32,
        definition: Definition,
        mut files: Vec<ProtectedFile>,
        checks: SealChecks,
        sealed_at: i64,
    ) -> Result<Self, SealRefused> {
        let problems = definition.problems();
        if !problems.is_empty() {
            return Err(SealRefused::Definition(problems));
        }
        files.sort();
        let seal = hash(&definition, &files)?;
        Ok(Self {
            experiment,
            version,
            definition,
            files,
            checks,
            seal,
            sealed_at,
        })
    }

    pub fn experiment(&self) -> ExperimentId {
        self.experiment
    }

    pub fn version(&self) -> u32 {
        self.version
    }

    pub fn definition(&self) -> &Definition {
        &self.definition
    }

    pub fn files(&self) -> &[ProtectedFile] {
        &self.files
    }

    pub fn checks(&self) -> &SealChecks {
        &self.checks
    }

    pub fn seal_hash(&self) -> &SealHash {
        &self.seal
    }

    pub fn sealed_at(&self) -> i64 {
        self.sealed_at
    }

    pub fn touched(&self, changed: &[(String, String)]) -> Vec<String> {
        self.definition.touched(changed)
    }

    pub fn knows_repos(&self, commits: &[CommitRef]) -> bool {
        self.definition.knows_repos(commits)
    }
}

fn hash(definition: &Definition, files: &[ProtectedFile]) -> Result<SealHash, SealRefused> {
    let bytes = canonical_json(&(definition, files)).map_err(|_| SealRefused::Unhashable)?;
    Ok(SealHash(hex(&Sha256::digest(bytes))))
}

/// `*` matches within one path segment, `**` across segments, `?` one character.
pub fn glob_matches(glob: &str, path: &str) -> bool {
    fn go(g: &[u8], p: &[u8]) -> bool {
        match g.split_first() {
            None => p.is_empty(),
            Some((b'*', rest)) => {
                if let Some(rest) = rest.strip_prefix(b"*") {
                    let rest = rest.strip_prefix(b"/").unwrap_or(rest);
                    (0..=p.len()).any(|i| p.get(i..).is_some_and(|tail| go(rest, tail)))
                } else {
                    let mut i = 0;
                    loop {
                        if p.get(i..).is_some_and(|tail| go(rest, tail)) {
                            return true;
                        }
                        match p.get(i) {
                            Some(b'/') | None => return false,
                            Some(_) => i += 1,
                        }
                    }
                }
            }
            Some((b'?', rest)) => match p.split_first() {
                Some((c, tail)) if *c != b'/' => go(rest, tail),
                _ => false,
            },
            Some((c, rest)) => match p.split_first() {
                Some((d, tail)) if c == d => go(rest, tail),
                _ => false,
            },
        }
    }
    go(glob.as_bytes(), path.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn globs() {
        assert!(glob_matches("bench/**", "bench/a/b.rs"));
        assert!(glob_matches("**/*.rs", "a/b/c.rs"));
        assert!(glob_matches("**/*.rs", "c.rs"));
        assert!(glob_matches("bench/*.rs", "bench/a.rs"));
        assert!(!glob_matches("bench/*.rs", "bench/x/a.rs"));
        assert!(!glob_matches("bench/**", "src/bench/a"));
        assert!(glob_matches("a?c", "abc"));
        assert!(!glob_matches("a?c", "a/c"));
        assert!(glob_matches("Makefile", "Makefile"));
        assert!(!glob_matches("Makefile", "Makefile2"));
    }

    pub fn definition() -> Definition {
        Definition {
            metric: Metric {
                name: "p95".into(),
                unit: "ms".into(),
                direction: Direction::Minimize,
            },
            kind: EvaluationKind::Speed,
            evaluation: Evaluation {
                command: "make bench".into(),
                repos: vec![RepoPin {
                    repo: "app".into(),
                    commit: "abc".into(),
                }],
                protected: vec![ProtectedPath {
                    repo: "app".into(),
                    glob: "bench/**".into(),
                }],
            },
            guardrails: vec![],
            data: DataRefs {
                evaluation: DatasetRef {
                    id: "d".into(),
                    version: 1,
                },
                holdout: DatasetRef {
                    id: "h".into(),
                    version: 1,
                },
            },
            repeats: 5,
            quick_check: true,
            patience: 6,
            holdout_cap: 20,
            value_model: None,
            criteria: CRITERIA_VERSION,
        }
    }

    pub fn checks() -> SealChecks {
        SealChecks::passed(
            100.0,
            2.0,
            STANDARD_CHEATS
                .iter()
                .map(|n| CheatCheck {
                    name: (*n).into(),
                    rejected: true,
                })
                .collect(),
        )
        .unwrap()
    }

    fn sealed() -> SealedExperiment {
        SealedExperiment::seal(ExperimentId::new(), 1, definition(), vec![], checks(), 0).unwrap()
    }

    #[test]
    fn seal_checks_refuse_what_fails() {
        let cheat = |n: &str, rejected| CheatCheck {
            name: n.into(),
            rejected,
        };
        let all = |rejected| {
            STANDARD_CHEATS
                .iter()
                .map(|n| cheat(n, rejected))
                .collect::<Vec<_>>()
        };
        assert!(matches!(
            SealChecks::passed(100.0, 2.0, all(false)),
            Err(SealRefused::CheatNotRejected(_))
        ));
        assert!(matches!(
            SealChecks::passed(100.0, 2.0, vec![]),
            Err(SealRefused::CheatMissing(_))
        ));
        assert_eq!(
            SealChecks::passed(1.0, 2.0, all(true)).unwrap_err(),
            SealRefused::TooNoisy
        );
        assert_eq!(
            SealChecks::passed(f64::NAN, 2.0, all(true)).unwrap_err(),
            SealRefused::NotFinite
        );
    }

    #[test]
    fn a_bad_definition_cannot_be_sealed() {
        let mut d = definition();
        d.repeats = 1;
        assert!(matches!(
            SealedExperiment::seal(ExperimentId::new(), 1, d, vec![], checks(), 0),
            Err(SealRefused::Definition(_))
        ));
    }

    #[test]
    fn a_tampered_seal_fails_to_load() {
        let json = serde_json::to_string(&sealed()).unwrap();
        assert!(serde_json::from_str::<SealedExperiment>(&json).is_ok());
        let tampered = json.replace("make bench", "make nothing");
        assert!(serde_json::from_str::<SealedExperiment>(&tampered).is_err());
    }

    #[test]
    fn protected_files_change_the_seal_and_touched_paths_are_found() {
        let file = ProtectedFile {
            repo: "app".into(),
            path: "bench/a".into(),
            sha256: "1".into(),
        };
        let a = SealedExperiment::seal(
            ExperimentId::new(),
            1,
            definition(),
            vec![file.clone()],
            checks(),
            0,
        )
        .unwrap();
        let b = SealedExperiment::seal(
            ExperimentId::new(),
            1,
            definition(),
            vec![ProtectedFile {
                sha256: "2".into(),
                ..file
            }],
            checks(),
            0,
        )
        .unwrap();
        assert_ne!(a.seal_hash(), b.seal_hash());
        let changed = vec![
            ("app".to_owned(), "src/x.rs".to_owned()),
            ("app".to_owned(), "bench/a.rs".to_owned()),
            ("other".to_owned(), "bench/a.rs".to_owned()),
        ];
        assert_eq!(a.touched(&changed), vec!["app:bench/a.rs".to_owned()]);
    }
}
