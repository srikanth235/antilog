# AWS in Ratchet

(target) Nothing here is on this branch yet. This page says how Ratchet runs on AWS when it lands: phase 1 Ratchet cloud, pay-per-use. Versions are in [libraries.md](../libraries.md).

For everything else about AWS, use the AWS agent skills (the Agent Toolkit for AWS, github.com/aws/agent-toolkit-for-aws, Apache-2.0) in your own Claude Code: `aws-serverless`, `aws-lambda-web-functions`, `aws-database`, `amazon-bedrock`, `agents-*`, `aws-iam`, `aws-secrets-manager`. They are not copied into this repo. **This page wins where they differ.** Its rules come from the product's: the one Rust server and one image, Postgres as the only store, nothing specific to one cloud outside an adapter ([../rust/ratchet.md](../rust/ratchet.md)).

## What gets built (target)

- **The server on Lambda.** The unchanged axum `Router` served through `lambda_http` and `lambda_runtime`, built with `cargo-lambda`, behind a Lambda function URL with response streaming. A native Linux binary: tokio, axum, `sqlx`, rustls and the AWS SDK all work, with no WebAssembly. No API Gateway, NAT gateway or load balancer.
- **Aurora Serverless v2 (PostgreSQL)** with a minimum of 0 ACUs and auto-pause, on an engine version that supports it. Idle cost is storage only.
- **A timer adapter** on EventBridge Scheduler (`aws-sdk-scheduler`), behind the core's timer trait.
- **The AgentCore adapter** in Rust, with `aws-sdk-bedrockagentcore` and `aws-sdk-bedrockagentcorecontrol`. Agent sessions and judge runs run on AgentCore, never on Lambda.

Kubernetes (EKS) runs the same image for a client who needs steady traffic: [../kubernetes/ratchet.md](../kubernetes/ratchet.md).

## Rules

- **Nothing runs after a response is sent.** Lambda freezes the process between requests: no spawned tokio task, in-process timer or background loop. Flush what you owe before you return.
- **Steps of about 10 minutes.** An invocation ends at 15 minutes. Long server work (relaying a session's events, waiting on a measurement or a watch) runs as a step that saves the platform's event cursor, then books its next wake-up. Lambda durable functions are not used: their SDK is TypeScript and Python only.
- **EventBridge Scheduler is the timer.** Due jobs stay rows in Postgres. Each due time books a one-off schedule that invokes the function; the core never sees the scheduler, only the trait. Wake-ups are at least once, so a handler is idempotent, keyed by the job id. Delete the schedule after it fires (use delete-after-completion on the schedule, and have the handler delete it as well), so schedules do not pile up.
- **Aurora pauses, and wakes slowly.** The first request after a pause waits about 15 seconds. Return a clear "waking up" state to the client rather than a timeout, and retry the connection with a backoff. Keep a minimum of 0 ACUs on Ratchet cloud's idle accounts; about $45 a month if kept on at 0.5 ACU.
- **No RDS Proxy.** It has a fixed hourly fee. Use a small pool per Lambda instance (one or two connections), test a connection before use, a short idle timeout, and a cap on the function's concurrency so instances times pool size stays under the database's connection limit.
- **The AWS agent skills' AgentCore guidance assumes Python agents.** It helps for the agent image we ship, not for our adapter, which is Rust and goes through the two SDK crates above.
- **One IAM role per function**, with only what that function calls: its database secret, the scheduler role it passes, the AgentCore actions. No wildcard resources, no stored access keys.
- **Secrets come from Secrets Manager** into the function's environment or a mounted value at deploy time, and are wrapped in a redacting type ([../rust/ratchet.md](../rust/ratchet.md)). The server never calls a secret-manager API at request time.
- **Only CI deploys.** No one runs `tofu apply` or `cargo lambda deploy` from a laptop; a plan is reviewed before an apply ([../security/ratchet.md](../security/ratchet.md)).
- **A budget alert on the account,** set before the first deploy, so a runaway loop or a retry storm is seen within hours.

## Check

```bash
cargo lambda build --release --arm64           # (target) the Lambda binary
cargo lambda watch                             # (target) then, in another shell:
cargo lambda invoke ratchet-server --data-file tests/events/health.json   # (target)
tofu -chdir=deploy/tofu/aws validate           # (target)
```

CI runs the same beside the chart's k3s install ([../../land/ci.md](../../land/ci.md)).

## Pitfalls

- **A task spawned "to finish in the background"** is frozen when the response returns, and may resume in a later request or never. Do the work first, or book a wake-up.
- **A pool sized for a long-lived server** exhausts Aurora when Lambda scales out. Count instances times pool size.
- **A schedule left behind** fires again, or fills the account's quota. Delete it after it fires.
- **A handler that is not idempotent** double-charges a budget or starts two sessions on the second wake-up.
- **A stream held to the cap** is cut mid-event. End it earlier and let the browser reconnect with `Last-Event-ID`.
- **NAT gateways, load balancers and RDS Proxy cost by the hour** whether or not anyone uses them. Leave them out of the pay-per-use setup.
