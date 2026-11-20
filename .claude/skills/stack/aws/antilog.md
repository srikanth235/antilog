# AWS in Antilog

> v0 retires AgentCore, the EventBridge Scheduler timer and Kubernetes as a requirement ([docs/V0.md](../../../../docs/V0.md)); the Lambda entry, Aurora, S3, CloudFront and the origin guard stay. `/internal/` is open only to the deploy workflow (`antilog_aws::lambda::DEPLOY_DOMAIN`).

This page says how Antilog runs on AWS: phase 1 Antilog cloud, pay-per-use. The code is on this branch and has been checked locally and with stubbed clients; **nothing has been applied to an account yet** (see "Before the first deploy"). Versions are in [libraries.md](../libraries.md).

For everything else about AWS, use the AWS agent skills (the Agent Toolkit for AWS, github.com/aws/agent-toolkit-for-aws, Apache-2.0) in your own Claude Code: `aws-serverless`, `aws-lambda-web-functions`, `aws-database`, `amazon-bedrock`, `agents-*`, `aws-iam`, `aws-secrets-manager`. They are not copied into this repo. **This page wins where they differ.** Its rules come from the product's: the one Rust server and one image, Postgres as the only store, nothing specific to one cloud outside an adapter ([../rust/antilog.md](../rust/antilog.md)).

## What is built

- **Region: `us-east-1`** for everything on Antilog cloud (owner's call, 17 Nov 2026). Check that each service used is offered there before relying on it.

- **The server on Lambda** (`crates/antilog-aws`, `lambda::serve(router)`). The unchanged axum `Router` served through `lambda_http` with response streaming, built with `cargo-lambda` as the binary `antilog-lambda` (arm64, `provided.al2023`), behind a Lambda function URL. A native Linux binary: tokio, axum, `sqlx`, rustls and the AWS SDK all work, with no WebAssembly. No API Gateway, NAT gateway or load balancer. The binary builds its pool lazily (one connection, so a paused Aurora does not delay start-up) and exposes `POST /internal/migrate`.
- **Aurora Serverless v2 (PostgreSQL)** (`deploy/tofu/aws/database.tf`) with a minimum of 0 ACUs and auto-pause, engine 18.x (read from the account: 17.9 and later support 0 ACUs). Idle cost is storage only. It is publicly reachable over forced TLS because a function outside a VPC has no fixed address and a NAT gateway costs by the hour; narrow `database_ingress_cidrs` if the function ever moves into a VPC.
- **Only CloudFront reaches the server.** The function URL is public, so the server refuses any request without the secret header `x-antilog-origin` (`antilog_server::origin::OriginGuard`, checked in `antilog_aws::lambda::handle` before the router), in constant time, with 403. CloudFront adds it to `/api/*` (a custom origin header). OpenTofu generates two values and keeps them in Secrets Manager (`antilog/origin-secret`); the function accepts both (`ANTILOG_ORIGIN_SECRET`, `ANTILOG_ORIGIN_SECRET_NEXT`) so one can be rotated while the other works (steps in `deploy/tofu/aws/README.md`). The check is always on in the Lambda binary and fails closed: with no secret set it refuses everything except the the deploy's direct invokes, and there is no `/health` exception (a local `cargo lambda invoke` sends the header from `tests/events/health.json` and sets `ANTILOG_ORIGIN_SECRET=local-origin-secret`). The local server (`antilog-server`) checks only when `ANTILOG_ORIGIN_SECRET` is set; tests build `OriginGuard::off()` explicitly.
- **Secrets** need no adapter: `antilog_core::secrets::EnvSecrets` reads `ANTILOG_SECRET_<NAME>` or a file in `ANTILOG_SECRETS_DIR`.
- **The infrastructure** is OpenTofu in `deploy/tofu/aws/` (state in S3 with a lock file, created once by `bootstrap/`), deployed only by `.github/workflows/deploy-aws.yml`. It holds the function and its URL, Aurora, S3 and CloudFront (the API is under `/api` on the same host), Secrets Manager entries, one role per function named `antilog-*` with the permissions boundary, and the existing `antilog-dev` budget as data. Customers bring their own model keys and the proxy meters spend, so there are no model-spend alarms.


## Rules

- **Nothing runs after a response is sent.** Lambda freezes the process between requests: no spawned tokio task, in-process timer or background loop. Flush what you owe before you return.
- **Aurora pauses, and wakes slowly.** The first request after a pause waits about 15 seconds. Return a clear "waking up" state to the client rather than a timeout, and retry the connection with a backoff. Keep a minimum of 0 ACUs on Antilog cloud's idle accounts; about $45 a month if kept on at 0.5 ACU.
- **No RDS Proxy.** It has a fixed hourly fee. Use a small pool per Lambda instance (one or two connections), test a connection before use, a short idle timeout, and a cap on the function's concurrency so instances times pool size stays under the database's connection limit.
- **One IAM role per function**, with only what that function calls: its database secret. No wildcard resources, no stored access keys.
- **Secrets come from Secrets Manager** into the function's environment or a mounted value at deploy time, and are wrapped in a redacting type ([../rust/antilog.md](../rust/antilog.md)). The server never calls a secret-manager API at request time.
- **Only CI deploys.** No one runs `tofu apply` or `cargo lambda deploy` from a laptop; a plan is reviewed before an apply ([../security/antilog.md](../security/antilog.md)).
- **A budget alert on the account,** set before the first deploy, so a runaway loop or a retry storm is seen within hours.

## Check

```bash
cargo lambda build --release --arm64 -p antilog-aws
# a local invoke needs a Postgres, SQLX_OFFLINE=true (a set DATABASE_URL makes sqlx check queries at build time),
# and placeholder values for the variables the `lambda` job of checks.yml sets (origin secret, receipt key)
SQLX_OFFLINE=true DATABASE_URL=postgres://... cargo lambda watch -A 127.0.0.1 -P 9000 -p antilog-aws --bin antilog-lambda
cargo lambda invoke -A 127.0.0.1 -P 9000 antilog-lambda --data-file crates/antilog-aws/tests/events/health.json
tofu fmt -check -recursive deploy/tofu
tofu -chdir=deploy/tofu/aws init -backend=false && tofu -chdir=deploy/tofu/aws validate
```

CI runs the same in the `lambda` and `tofu` jobs of `checks.yml` ([../../land/ci.md](../../land/ci.md)). A `tofu plan` needs AWS credentials (read-only calls) and runs in `deploy-aws.yml`, in the `aws-dev` environment.

## Decisions to review

- **No reserved concurrency yet.** The account's Lambda quota is 10 and Lambda keeps 10 unreserved, so `server_reserved_concurrency` stays null until the quota is raised.
- **Aurora is public** (see above) because of the no-NAT rule. The protection is the generated password and forced TLS.
- **A reload on a screen of the app** is answered by a viewer-request CloudFront Function on the web (S3) behavior only (`deploy/tofu/aws/functions/spa-fallback.js`: a path with no file extension becomes `/index.html`; test: `node --test deploy/tofu/aws/functions`). There is no distribution-wide error page, because that would turn an API 403 or 404 into HTML with status 200.
- **The web app is served under `/api`** on the CloudFront host with the prefix stripped, because the API's routes have no common prefix and share paths with the app's screens. The web client's base URL must be `/api`.
- **A pull request is checked without AWS** and planned only with a role that trusts pull requests; `antilog-ci` trusts the `aws-dev` environment alone.
- **A runtime that loses its microVM loses its event log.** The agent image keeps the log in memory; the spike measures how often that matters.

## Before the first deploy (the owner)

1. Create the GitHub environment `aws-dev` with required reviewers, and the repository variable `AWS_PLAN_ROLE_ARN` only if a pull-request role is made.
2. Run `bootstrap/` once with owner credentials to create the state bucket `antilog-tfstate-987432215914`.
3. Create the ACM certificate in `us-east-1` for the app's name, set `domain_name` and `certificate_arn`, and add a DNS-only CNAME at Cloudflare to the `web_domain` output (no DNS resources are in the module).
4. Put the GitHub OAuth app's client secret in Secrets Manager (`antilog/github-client-secret`) after the first apply, set `github_client_id` and `signin_secrets_populated`, then set `public_url` to the `web_url` output and apply again. The OAuth app's callback URL is `https://<domain>/auth/github/callback`: the web app's own route, which POSTs the code to the API's `POST /auth/github/callback` (never `/api/...`, which only accepts the POST).
5. Ask AWS for a Lambda concurrency quota increase, then set `server_reserved_concurrency`.
6. Build and push the agent and judge images, then set `agent_image_uri` and `judge_image_uri`.
7. Run `deploy-aws` by hand: plan, review, then apply. The workflow migrates the database after the apply.

## Pitfalls

- **A task spawned "to finish in the background"** is frozen when the response returns, and may resume in a later request or never. Do the work first, or book a wake-up.
- **A pool sized for a long-lived server** exhausts Aurora when Lambda scales out. Count instances times pool size.
- **A schedule left behind** fires again, or fills the account's quota. Delete it after it fires.
- **A handler that is not idempotent** double-charges a budget or starts two sessions on the second wake-up.
- **A stream held to the cap** is cut mid-event. End it earlier and let the browser reconnect with `Last-Event-ID`. The live-update stream (`GET /api/events/stream`) ends after 4 minutes and sends a comment every 10; CloudFront has its own `/api/events/*` behavior with caching and compression off. Not yet proven end to end on a real Lambda and CloudFront: the first deploy checks that `curl -N` prints `event: hello` at once.
- **NAT gateways, load balancers and RDS Proxy cost by the hour** whether or not anyone uses them. Leave them out of the pay-per-use setup.
