//! Customize, Evaluations: templates an experiment may start from.

use antilog_types::{AuditOutcome, EvaluationTemplate, NewEvaluationTemplate};

use crate::organization::{Action, Principal, Refusal, entry, member_of, permit};
use crate::store::Store;

/// "Load test (p95)" becomes `load-test-p95`.
pub fn slug(name: &str) -> String {
    let mut out = String::new();
    for c in name.trim().chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_end_matches('-').chars().take(64).collect()
}

pub async fn list<S: Store>(
    store: &S,
    who: &Principal,
) -> Result<Vec<EvaluationTemplate>, Refusal> {
    member_of(store, who).await?;
    Ok(store.evaluations(who.organization).await?)
}

pub async fn get<S: Store>(
    store: &S,
    who: &Principal,
    id: &str,
) -> Result<EvaluationTemplate, Refusal> {
    list(store, who)
        .await?
        .into_iter()
        .find(|template| template.id == id)
        .ok_or(Refusal::NotFound)
}

/// Editors and owners add one. The id comes from the name; a taken one is a conflict.
pub async fn add<S: Store>(
    store: &S,
    who: &Principal,
    now: i64,
    new: NewEvaluationTemplate,
) -> Result<EvaluationTemplate, Refusal> {
    let id = slug(&new.name);
    let text = |value: &str, max: usize| (1..=max).contains(&value.trim().chars().count());
    if id.is_empty()
        || !text(&new.name, 200)
        || new.summary.chars().count() > 500
        || !text(&new.metric, 200)
        || !text(&new.command, 2000)
        || new.protected_paths.len() > 100
        || new.protected_paths.iter().any(|path| !text(path, 300))
    {
        return Err(Refusal::Invalid(
            "an evaluation needs a name, a metric and a command, within the length limits",
        ));
    }
    permit(store, who, Action::EvaluationAdd, None, &id, now).await?;
    let template = EvaluationTemplate {
        id: id.clone(),
        name: new.name.trim().to_owned(),
        summary: new.summary.trim().to_owned(),
        kind: new.kind,
        metric: new.metric.trim().to_owned(),
        lower_is_better: new.lower_is_better,
        command: new.command.trim().to_owned(),
        protected_paths: new
            .protected_paths
            .iter()
            .map(|path| path.trim().to_owned())
            .collect(),
        created_by: who.member,
        created_at: now,
    };
    store
        .add_evaluation(
            &template,
            who.organization,
            &entry(who, "evaluation.add", &id, AuditOutcome::Done, now),
        )
        .await?;
    Ok(template)
}
