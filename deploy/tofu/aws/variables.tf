variable "region" {
  description = "Everything runs in one region (owner's ruling, 17 Nov 2026)."
  type        = string
  default     = "us-east-1"
}

variable "account_id" {
  description = "The AWS account this module may change."
  type        = string
  default     = "987432215914"
}

variable "name" {
  description = "Prefix for every resource. IAM roles must start with antilog- (the CI policy only allows those)."
  type        = string
  default     = "antilog"

  validation {
    condition     = startswith(var.name, "antilog")
    error_message = "The name must start with antilog: the CI policy only manages antilog-* roles."
  }
}

variable "environment" {
  type    = string
  default = "dev"
}

variable "permissions_boundary_arn" {
  description = "Attached to every role this module creates. Created by the owner, not by this module."
  type        = string
  default     = "arn:aws:iam::987432215914:policy/antilog-boundary"
}

variable "budget_name" {
  description = "The existing monthly budget, read as data and never recreated."
  type        = string
  default     = "antilog-dev"
}

# The server function ----------------------------------------------------------

variable "server_zip" {
  description = "The zip `cargo lambda build --release --arm64 --output-format zip -p antilog-aws` writes."
  type        = string
  default     = "../../../target/lambda/antilog-lambda/bootstrap.zip"
}

variable "server_memory_mb" {
  type    = number
  default = 512
}

variable "server_timeout_seconds" {
  description = "A relay step is about 10 minutes and an invocation ends at 15."
  type        = number
  default     = 840
}

variable "server_reserved_concurrency" {
  description = <<-EOT
    Reserved concurrency for the server function, or null for none. A new account's quota
    is 10 and Lambda keeps 10 unreserved, so any positive number fails until the quota is
    raised. Set it (to about 5 instances times one connection each, under the database's
    limit) after the increase. The kill switch sets it to 0 regardless.
  EOT
  type        = number
  default     = null
}

variable "log_retention_days" {
  type    = number
  default = 14
}

# The database -----------------------------------------------------------------

variable "database_engine_version" {
  description = "An Aurora PostgreSQL version that supports a minimum of 0 ACUs."
  type        = string
  default     = "18.6"
}

variable "database_max_acu" {
  type    = number
  default = 2
}

variable "database_seconds_until_auto_pause" {
  type    = number
  default = 300
}

variable "database_ingress_cidrs" {
  description = <<-EOT
    Who may reach port 5432. The function runs outside a VPC (no NAT gateway), so its
    addresses are not fixed and this stays open; the password and TLS (forced by the
    cluster) are the protection. Narrow it if the function ever moves into a VPC.
  EOT
  type        = list(string)
  default     = ["0.0.0.0/0"]
}

variable "database_deletion_protection" {
  type    = bool
  default = true
}

# Sign-in ----------------------------------------------------------------------

variable "github_client_id" {
  description = "The GitHub OAuth app's client id (not a secret). Empty turns GitHub sign-in off."
  type        = string
  default     = ""
}

variable "origin_secret_active" {
  description = "Which of the two generated origin secrets (\"a\" or \"b\") CloudFront sends. The function accepts both. Rotate by replacing the other one, then switching this."
  type        = string
  default     = "a"

  validation {
    condition     = contains(["a", "b"], var.origin_secret_active)
    error_message = "origin_secret_active is \"a\" or \"b\"."
  }
}

variable "signin_secrets_populated" {
  description = "Set true once the owner has put the GitHub client secret in its Secrets Manager entry."
  type        = bool
  default     = false
}

# Spend guards -----------------------------------------------------------------

variable "bedrock_hourly_limit_usd" {
  description = "Model spend in one hour that trips the kill switch."
  type        = number
  default     = 5
}

variable "bedrock_daily_limit_usd" {
  description = "Model spend in one day that trips the kill switch."
  type        = number
  default     = 15
}

variable "public_url" {
  description = <<-EOT
    The https address people reach the web app at, without a trailing slash (the CloudFront
    domain from the outputs, or a custom domain). Sign-in redirects are built from it. It
    cannot be read from the distribution because the distribution depends on the function:
    apply once, copy the `web_url` output here, apply again.
  EOT
  type        = string
  default     = ""
}

variable "killswitch_role_name" {
  description = <<-EOT
    The role the kill-switch function runs as. The owner creates it OUTSIDE antilog-* (like
    budget-stop-executor), so the permissions boundary does not cap it and the deny-all it
    writes cannot lock it. Its policy is listed in deploy/tofu/aws/README.md.
  EOT
  type        = string
  default     = "kill-switch-executor"
}

variable "domain_name" {
  description = "A name for the web app, such as dev.antilog.tech. DNS is managed by hand at Cloudflare: add a DNS-only CNAME to the web_domain output. Empty uses the *.cloudfront.net name."
  type        = string
  default     = ""
}

variable "certificate_arn" {
  description = "An ACM certificate in us-east-1 covering domain_name, created by the owner. Required with domain_name."
  type        = string
  default     = ""

  validation {
    condition     = var.certificate_arn == "" || startswith(var.certificate_arn, "arn:aws:acm:us-east-1:")
    error_message = "CloudFront only takes certificates from us-east-1."
  }
}
