output "web_url" {
  description = "Where people reach the app. Copy it into public_url and apply again."
  value       = "https://${aws_cloudfront_distribution.web.domain_name}"
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

output "ecr_agent_repository" {
  value = aws_ecr_repository.agent.repository_url
}

output "ecr_judge_repository" {
  value = aws_ecr_repository.judge.repository_url
}

output "killswitch_function" {
  value = aws_lambda_function.killswitch.function_name
}

output "budget_limit_usd" {
  description = "The existing monthly budget this stack sits under"
  value       = data.aws_budgets_budget.dev.budget_limit[0].amount
}
