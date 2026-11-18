# Antilog cloud on AWS (OpenTofu)

Rules and decisions are in [`.claude/skills/stack/aws/antilog.md`](../../../.claude/skills/stack/aws/antilog.md). Only `.github/workflows/deploy-aws.yml` applies this module.

## The kill-switch role (the owner creates it, once)

`kill-switch-executor` sits outside `antilog-*`, so the permissions boundary does not cap it and the deny-all it writes cannot lock it (the same shape as `budget-stop-executor`). Trust: `lambda.amazonaws.com`. Policy, with the account and names of this module:

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {"Effect": "Allow", "Action": ["logs:CreateLogStream", "logs:PutLogEvents"],
     "Resource": "arn:aws:logs:us-east-1:987432215914:log-group:/aws/lambda/antilog-killswitch:*"},
    {"Effect": "Allow", "Action": ["iam:PutRolePolicy", "iam:DeleteRolePolicy"],
     "Resource": [
       "arn:aws:iam::987432215914:role/antilog-server",
       "arn:aws:iam::987432215914:role/antilog-scheduler",
       "arn:aws:iam::987432215914:role/antilog-agent-runtime",
       "arn:aws:iam::987432215914:role/antilog-judge-runtime"]},
    {"Effect": "Allow", "Action": ["lambda:PutFunctionConcurrency", "lambda:DeleteFunctionConcurrency"],
     "Resource": "arn:aws:lambda:us-east-1:987432215914:function:antilog-server"},
    {"Effect": "Allow", "Action": ["scheduler:GetSchedule", "scheduler:UpdateSchedule"],
     "Resource": "arn:aws:scheduler:us-east-1:987432215914:schedule/antilog/*"},
    {"Effect": "Allow", "Action": "scheduler:ListSchedules", "Resource": "*"},
    {"Effect": "Allow", "Action": "iam:PassRole",
     "Resource": "arn:aws:iam::987432215914:role/antilog-scheduler",
     "Condition": {"StringEquals": {"iam:PassedToService": "scheduler.amazonaws.com"}}}
  ]
}
```

The CI role also needs `iam:PassRole` on `role/kill-switch-executor` (the `antilog-dev` policy only passes `antilog-*`), and `iam:GetRole` already covers the lookup.

## Domain

DNS for `antilog.tech` is at Cloudflare and is managed by hand; this module has no DNS, Route 53 or Cloudflare resources. The owner creates the ACM certificate in `us-east-1` for the names in use, then sets `domain_name` and `certificate_arn`, applies, and adds a DNS-only CNAME from the name to the `web_domain` output.

## Only CloudFront reaches the server

The Lambda function URL is public, so the server checks for itself. CloudFront adds `x-antilog-origin: <secret>` to every `/api/*` request (a custom origin header, which replaces any value the browser sends), and the function refuses every request without it with 403, except EventBridge Scheduler's direct invokes. There is no exception for `/health` (it answers through `/api/health`). With no secret in its environment the function refuses everything: it fails closed. The check cannot be turned off on Lambda.

OpenTofu generates two values (`random_password.origin["a"]` and `["b"]`) and keeps them in the Secrets Manager entry `antilog/origin-secret` (`{"active": ..., "other": ...}`). CloudFront sends the one named by `origin_secret_active`; the function accepts both (`ANTILOG_ORIGIN_SECRET` and `ANTILOG_ORIGIN_SECRET_NEXT`). Nothing is needed from the owner for the first deploy.

### Rotating the origin secret

1. Replace the value that is not active: `tofu apply -replace='random_password.origin["b"]'` when `a` is active. The function now accepts `a` and the new `b`; CloudFront still sends `a`.
2. Set `origin_secret_active = "b"` and apply. CloudFront switches to `b` (a few minutes to reach every edge; both are accepted meanwhile).
3. The old `a` still works until the next rotation replaces it. To end it now, replace `a` and apply.

Only `deploy-aws.yml` applies; run these as inputs to that workflow, not from a laptop.

### Checking it

`curl -i https://<function url>/health` answers 403, and `curl -i https://<app host>/api/health` answers 200. The `lambda` CI job calls the function with the header in `crates/antilog-aws/tests/events/health.json` and the placeholder `ANTILOG_ORIGIN_SECRET=local-origin-secret`.
