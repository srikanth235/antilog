# The server: the one axum Router on Lambda, behind a function URL with response streaming.
# No API Gateway, load balancer or NAT gateway. Built by
#   cargo lambda build --release --arm64 --output-format zip -p antilog-aws

data "aws_caller_identity" "current" {}

resource "aws_cloudwatch_log_group" "server" {
  name              = "/aws/lambda/${local.server_function}"
  retention_in_days = var.log_retention_days
}

resource "aws_iam_role" "server" {
  name                 = "${local.prefix}-server"
  description          = "The server function: logs, its schedules, and the agent runtimes it calls"
  permissions_boundary = var.permissions_boundary_arn

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Effect    = "Allow"
      Principal = { Service = "lambda.amazonaws.com" }
      Action    = "sts:AssumeRole"
    }]
  })
}

locals {
  runtime_arns = compact([
    try(aws_bedrockagentcore_agent_runtime.agent[0].agent_runtime_arn, ""),
    try(aws_bedrockagentcore_agent_runtime.judge[0].agent_runtime_arn, ""),
  ])
}

resource "aws_iam_role_policy" "server" {
  name = "server"
  role = aws_iam_role.server.id

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = concat(
      [
        {
          Sid      = "Logs"
          Effect   = "Allow"
          Action   = ["logs:CreateLogStream", "logs:PutLogEvents"]
          Resource = "${aws_cloudwatch_log_group.server.arn}:*"
        },
        {
          Sid      = "BookWakeUps"
          Effect   = "Allow"
          Action   = ["scheduler:CreateSchedule", "scheduler:UpdateSchedule", "scheduler:DeleteSchedule", "scheduler:GetSchedule"]
          Resource = "arn:aws:scheduler:${var.region}:${var.account_id}:schedule/${aws_scheduler_schedule_group.wake.name}/*"
        },
        {
          Sid       = "HandTheSchedulerItsRole"
          Effect    = "Allow"
          Action    = "iam:PassRole"
          Resource  = aws_iam_role.scheduler.arn
          Condition = { StringEquals = { "iam:PassedToService" = "scheduler.amazonaws.com" } }
        },
      ],
      length(local.runtime_arns) == 0 ? [] : [
        {
          Sid    = "CallTheAgentRuntimes"
          Effect = "Allow"
          Action = ["bedrock-agentcore:InvokeAgentRuntime", "bedrock-agentcore:StopRuntimeSession"]
          Resource = flatten([
            for arn in local.runtime_arns : [arn, "${arn}/runtime-endpoint/*"]
          ])
        },
      ],
    )
  })
}

resource "aws_lambda_function" "server" {
  function_name = local.server_function
  description   = "Antilog server: the axum Router on Lambda"
  role          = aws_iam_role.server.arn

  runtime       = "provided.al2023"
  architectures = ["arm64"]
  handler       = "bootstrap"
  memory_size   = var.server_memory_mb
  timeout       = var.server_timeout_seconds

  filename         = var.server_zip
  source_code_hash = fileexists(var.server_zip) ? filebase64sha256(var.server_zip) : null

  reserved_concurrent_executions = var.server_reserved_concurrency

  environment {
    variables = merge(
      {
        RUST_LOG                     = "info"
        ANTILOG_SECRET_DATABASE_URL  = aws_secretsmanager_secret_version.database_url.secret_string
        ANTILOG_WAKE_SECRET          = aws_secretsmanager_secret_version.wake.secret_string
        ANTILOG_WAKE_SECRET          = aws_secretsmanager_secret_version.wake.secret_string
        ANTILOG_SECRET_RECEIPT_KEY   = aws_secretsmanager_secret_version.receipt_key.secret_string
        ANTILOG_SCHEDULER_GROUP      = aws_scheduler_schedule_group.wake.name
        ANTILOG_SCHEDULER_TARGET_ARN = "arn:aws:lambda:${var.region}:${var.account_id}:function:${local.server_function}"
        ANTILOG_SCHEDULER_ROLE_ARN   = aws_iam_role.scheduler.arn
        ANTILOG_AGENT_MODEL          = local.agent_model
        ANTILOG_AGENT_FALLBACK_MODEL = local.fallback_model
      },
      length(aws_bedrockagentcore_agent_runtime.agent) == 0 ? {} : {
        ANTILOG_AGENTCORE_AGENT_RUNTIME = aws_bedrockagentcore_agent_runtime.agent[0].agent_runtime_arn
      },
      length(aws_bedrockagentcore_agent_runtime.judge) == 0 ? {} : {
        ANTILOG_AGENTCORE_JUDGE_RUNTIME = aws_bedrockagentcore_agent_runtime.judge[0].agent_runtime_arn
      },
      var.github_client_id == "" || var.public_url == "" ? {} : {
        ANTILOG_GITHUB_CLIENT_ID = var.github_client_id
        ANTILOG_GITHUB_REDIRECT  = "${var.public_url}/api/auth/github/callback"
      },
      length(data.aws_secretsmanager_secret_version.github_client_secret) == 0 ? {} : {
        ANTILOG_GITHUB_CLIENT_SECRET = data.aws_secretsmanager_secret_version.github_client_secret[0].secret_string
      },
    )
  }

  logging_config {
    log_format = "Text"
    log_group  = aws_cloudwatch_log_group.server.name
  }

  depends_on = [aws_iam_role_policy.server]
}

resource "aws_lambda_function_url" "server" {
  function_name      = aws_lambda_function.server.function_name
  authorization_type = "NONE"
  invoke_mode        = "RESPONSE_STREAM"
}

# A public function URL needs both permissions since October 2025.
resource "aws_lambda_permission" "url" {
  statement_id           = "AllowPublicFunctionUrl"
  function_name          = aws_lambda_function.server.function_name
  action                 = "lambda:InvokeFunctionUrl"
  principal              = "*"
  function_url_auth_type = "NONE"
}

resource "aws_lambda_permission" "url_invoke" {
  statement_id             = "AllowPublicFunctionUrlInvoke"
  function_name            = aws_lambda_function.server.function_name
  action                   = "lambda:InvokeFunction"
  principal                = "*"
  invoked_via_function_url = true
}
