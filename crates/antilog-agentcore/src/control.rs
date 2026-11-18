//! Runtime names to ARNs, through the control plane, read-only.

use antilog_core::platform::PlatformError;

/// An ARN stays as it is; a name is looked up in the account's runtimes.
pub async fn resolve_runtime_arn(
    control: &aws_sdk_bedrockagentcorecontrol::Client,
    name_or_arn: &str,
) -> Result<String, PlatformError> {
    if name_or_arn.starts_with("arn:") {
        return Ok(name_or_arn.to_owned());
    }
    let mut pages = control.list_agent_runtimes().into_paginator().send();
    while let Some(page) = pages.next().await {
        let page = page.map_err(|e| PlatformError::Unavailable(Box::new(e)))?;
        if let Some(found) = page
            .agent_runtimes()
            .iter()
            .find(|r| r.agent_runtime_name() == name_or_arn)
        {
            return Ok(found.agent_runtime_arn().to_owned());
        }
    }
    Err(PlatformError::Failed(format!(
        "no AgentCore runtime is named {name_or_arn}"
    )))
}
