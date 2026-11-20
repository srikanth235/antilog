output "web_domain" {
  description = "The CloudFront name. Add a DNS-only CNAME from domain_name to it at Cloudflare, by hand."
  value       = aws_cloudfront_distribution.web.domain_name
}

output "web_url" {
  description = "Where people reach the app. Copy it into public_url and apply again."
  value       = var.domain_name == "" ? "https://${aws_cloudfront_distribution.web.domain_name}" : "https://${var.domain_name}"
}

output "web_bucket" {
  description = "The deploy workflow syncs web/dist here."
  value       = aws_s3_bucket.web.bucket
}

output "web_distribution_id" {
  value = aws_cloudfront_distribution.web.id
}

output "server_function" {
  description = "The deploy workflow updates this function's code and calls POST /internal/migrate."
  value       = aws_lambda_function.server.function_name
}

output "database_endpoint" {
  value = aws_rds_cluster.database.endpoint
}

output "killswitch_function" {
  value = aws_lambda_function.killswitch.function_name
}

output "budget_limit_usd" {
  description = "The existing monthly budget this stack sits under"
  value       = data.aws_budgets_budget.dev.budget_limit[0].amount
}
