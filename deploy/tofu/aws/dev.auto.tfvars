# The dev environment's settings. Nothing secret goes here: secrets live in Secrets Manager.
# OpenTofu loads this file on every plan and apply in this directory.

domain_name     = "dev.antilog.tech"
certificate_arn = "arn:aws:acm:us-east-1:987432215914:certificate/8711bf64-ef80-4fe7-9740-18bdaba1a0b9"
public_url      = "https://dev.antilog.tech"

# Set once the GitHub OAuth app exists (its client id is not a secret), then set
# signin_secrets_populated = true after its client secret is in Secrets Manager.
# github_client_id         = ""
# signin_secrets_populated = false
