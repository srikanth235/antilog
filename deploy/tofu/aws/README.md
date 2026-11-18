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
