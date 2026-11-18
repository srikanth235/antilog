#![expect(
    clippy::expect_used,
    reason = "test support: a failed call fails the suite"
)]

//! The contract every [`Platform`] adapter meets for the agent half of the trait.
//! The fake runs it, and so does each real adapter, against a stubbed client.

use std::time::Duration;

use serde_json::json;

use crate::platform::{
    Platform, PlatformError, PlatformEvent, SessionEvents, SessionHandle, SessionSpec, ToolDef,
};

fn spec() -> SessionSpec {
    SessionSpec {
        agent: "coder".to_owned(),
        prompt: "make it faster".to_owned(),
        repos: vec!["acme/app".to_owned()],
        environment: None,
        tools: vec![ToolDef {
            name: "measure".to_owned(),
            description: "measure a trial".to_owned(),
            schema: json!({"type": "object"}),
        }],
    }
}

async fn poll<P: Platform>(
    platform: &P,
    handle: &SessionHandle,
    cursor: Option<&str>,
) -> SessionEvents {
    platform
        .session_events(handle, cursor, Duration::ZERO)
        .await
        .expect("events are readable")
}

/// Runs the agent half of the contract.
pub async fn agent_suite<P: Platform>(platform: &P) {
    assert!(!platform.name().is_empty(), "a platform has a name");

    // Start, then read from the beginning.
    let handle = platform.start_session(&spec()).await.expect("starts");
    let first = poll(platform, &handle, None).await;
    assert!(!first.events.is_empty(), "a started session says something");
    let cursor = first.next_cursor.clone();

    // The cursor moves past what was read: reading again repeats nothing.
    let again = poll(platform, &handle, cursor.as_deref()).await;
    assert!(
        again.events.is_empty(),
        "events after the cursor are only new ones, got {:?}",
        again.events
    );

    // Input and a tool answer are accepted, and answering twice is harmless.
    platform
        .send_input(&handle, "try the cache")
        .await
        .expect("input is accepted");
    for _ in 0..2 {
        platform
            .answer_tool(&handle, "call-1", &json!({"ok": true}))
            .await
            .expect("an answer is accepted once and again");
    }

    // A stopped session ends.
    platform.stop(&handle).await.expect("stops");
    let mut ended = false;
    let mut at = cursor;
    for _ in 0..5 {
        let page = poll(platform, &handle, at.as_deref()).await;
        ended |= page
            .events
            .iter()
            .any(|e| matches!(e, PlatformEvent::Ended { .. }));
        at = page.next_cursor.or(at);
        if ended {
            break;
        }
    }
    assert!(ended, "a stopped session reports Ended");

    // Delete removes it, twice is fine, and it is gone.
    platform.delete(&handle).await.expect("deletes");
    platform
        .delete(&handle)
        .await
        .expect("deleting twice is fine");
    assert!(
        matches!(
            platform.session_events(&handle, None, Duration::ZERO).await,
            Err(PlatformError::UnknownSession)
        ),
        "a deleted session is unknown"
    );

    // A handle nobody started is unknown, not unavailable.
    let stranger = SessionHandle("never-started".to_owned());
    assert!(matches!(
        platform.send_input(&stranger, "hello").await,
        Err(PlatformError::UnknownSession)
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fakes::FakePlatform;

    #[tokio::test]
    async fn the_fake_meets_the_contract() {
        agent_suite(&FakePlatform::default()).await;
    }
}
