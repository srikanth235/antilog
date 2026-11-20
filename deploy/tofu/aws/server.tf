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
  description          = "The server function: its logs"
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

resource "aws_iam_role_policy" "server" {
  name = "server"
  role = aws_iam_role.server.id

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Sid      = "Logs"
        Effect   = "Allow"
        Action   = ["logs:CreateLogStream", "logs:PutLogEvents"]
        Resource = "${aws_cloudwatch_log_group.server.arn}:*"
      },
    ]
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
        RUST_LOG                    = "info"
        ANTILOG_SECRET_DATABASE_URL = aws_secretsmanager_secret_version.database_url.secret_string
        ANTILOG_ORIGIN_SECRET       = local.origin_active
        ANTILOG_ORIGIN_SECRET_NEXT  = local.origin_next
        ANTILOG_SECRET_RECEIPT_KEY  = aws_secretsmanager_secret_version.receipt_key.secret_string
      },
      var.github_client_id == "" || var.public_url == "" ? {} : {
        ANTILOG_GITHUB_CLIENT_ID = var.github_client_id
        ANTILOG_GITHUB_REDIRECT  = "${var.public_url}/auth/github/callback"
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

# The URL is public (a function behind CloudFront cannot be private without IAM signing of
# POST bodies), so the server itself refuses every request that lacks CloudFront's
# `x-antilog-origin` secret; see secrets.tf. Only the deploy's direct invokes pass without it.
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
