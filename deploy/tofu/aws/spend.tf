# Spend guards, in code. The account's $50 monthly budget `antilog-dev` already exists and
# is read here as data, never recreated. Budgets tell a person within hours; the alarms
# below act within minutes: model spend priced from Bedrock's token counts for the two
# allowed models, and a kill-switch function the alarm calls directly (CloudWatch alarms
# can invoke a function without an SNS topic).

data "aws_budgets_budget" "dev" {
  name = var.budget_name
}

locals {
  model_ids = keys(local.models)

  # (input tokens * input price + output tokens * output price) per model, in dollars.
  spend_expression = "(${join(" + ", flatten([
    for i, id in local.model_ids : [
      "in${i} * ${local.models[id].input}",
      "out${i} * ${local.models[id].output}",
    ]
  ]))}) / 1000000"

  spend_alarms = {
    hourly = { period = 3600, limit = var.bedrock_hourly_limit_usd, window = "an hour" }
    daily  = { period = 86400, limit = var.bedrock_daily_limit_usd, window = "a day" }
  }
}

resource "aws_cloudwatch_metric_alarm" "model_spend" {
  for_each = local.spend_alarms

  alarm_name          = "${local.prefix}-model-spend-${each.key}"
  alarm_description   = "Bedrock spend on the two allowed models passed $${each.value.limit} in ${each.value.window}. Calls the kill switch."
  comparison_operator = "GreaterThanThreshold"
  threshold           = each.value.limit
  evaluation_periods  = 1
  treat_missing_data  = "notBreaching"
  alarm_actions       = [aws_lambda_function.killswitch.arn]

  metric_query {
    id          = "spend"
    expression  = local.spend_expression
    label       = "Model spend (USD)"
    return_data = true
  }

  dynamic "metric_query" {
    for_each = { for i, id in local.model_ids : "in${i}" => { model = id, metric = "InputTokenCount" } }
    content {
      id = metric_query.key
      metric {
        namespace   = "AWS/Bedrock"
        metric_name = metric_query.value.metric
        dimensions  = { ModelId = metric_query.value.model }
        period      = each.value.period
        stat        = "Sum"
      }
    }
  }

  dynamic "metric_query" {
    for_each = { for i, id in local.model_ids : "out${i}" => { model = id, metric = "OutputTokenCount" } }
    content {
      id = metric_query.key
      metric {
        namespace   = "AWS/Bedrock"
        metric_name = metric_query.value.metric
        dimensions  = { ModelId = metric_query.value.model }
        period      = each.value.period
        stat        = "Sum"
      }
    }
  }
}

# The kill switch -------------------------------------------------------------------
# Deny-all on the antilog-* roles it locks, the server's concurrency to 0, and every
# schedule disabled. It cannot lock itself or the CI role that releases it. AgentCore has
# no call that lists a runtime's sessions, so runtimes are stopped by denying their role
# (no more model calls, no image pulls) and by nothing being able to start a new session;
# a running session ends at its idle timeout.

data "archive_file" "killswitch" {
  type        = "zip"
  source_file = "${path.module}/killswitch/handler.py"
  output_path = "${path.module}/.build/killswitch.zip"
}

resource "aws_cloudwatch_log_group" "killswitch" {
  name              = "/aws/lambda/${local.killswitch}"
  retention_in_days = 90
}

resource "aws_iam_role" "killswitch" {
  name                 = "${local.prefix}-killswitch"
  description          = "Locks the other antilog roles when spend runs away"
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

resource "aws_iam_role_policy" "killswitch" {
  name = "killswitch"
  role = aws_iam_role.killswitch.id

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Sid      = "Logs"
        Effect   = "Allow"
        Action   = ["logs:CreateLogStream", "logs:PutLogEvents"]
        Resource = "${aws_cloudwatch_log_group.killswitch.arn}:*"
      },
      {
        Sid      = "DenyAllOnTheLockedRoles"
        Effect   = "Allow"
        Action   = ["iam:PutRolePolicy", "iam:DeleteRolePolicy"]
        Resource = [for name in local.locked_roles : "arn:aws:iam::${var.account_id}:role/${name}"]
      },
      {
        Sid      = "StopTheServerFromRunning"
        Effect   = "Allow"
        Action   = ["lambda:PutFunctionConcurrency", "lambda:DeleteFunctionConcurrency"]
        Resource = aws_lambda_function.server.arn
      },
      {
        Sid      = "DisableTheSchedules"
        Effect   = "Allow"
        Action   = ["scheduler:GetSchedule", "scheduler:UpdateSchedule"]
        Resource = "arn:aws:scheduler:${var.region}:${var.account_id}:schedule/${aws_scheduler_schedule_group.wake.name}/*"
      },
      {
        # ListSchedules cannot be limited to a group; it lists names only.
        Sid      = "ListTheSchedules"
        Effect   = "Allow"
        Action   = "scheduler:ListSchedules"
        Resource = "*"
      },
      {
        Sid       = "HandTheSchedulerItsRole"
        Effect    = "Allow"
        Action    = "iam:PassRole"
        Resource  = aws_iam_role.scheduler.arn
        Condition = { StringEquals = { "iam:PassedToService" = "scheduler.amazonaws.com" } }
      },
    ]
  })
}

resource "aws_lambda_function" "killswitch" {
  function_name = local.killswitch
  description   = "Spend kill switch: deny-all on antilog roles, concurrency 0, schedules off"
  role          = aws_iam_role.killswitch.arn

  runtime       = "python3.13"
  architectures = ["arm64"]
  handler       = "handler.handler"
  timeout       = 120
  memory_size   = 128

  filename         = data.archive_file.killswitch.output_path
  source_code_hash = data.archive_file.killswitch.output_base64sha256

  environment {
    variables = {
      LOCKED_ROLES   = join(",", local.locked_roles)
      SERVER         = local.server_function
      SCHEDULE_GROUP = aws_scheduler_schedule_group.wake.name
    }
  }

  logging_config {
    log_format = "Text"
    log_group  = aws_cloudwatch_log_group.killswitch.name
  }

  depends_on = [aws_iam_role_policy.killswitch]
}

resource "aws_lambda_permission" "alarms" {
  for_each = aws_cloudwatch_metric_alarm.model_spend

  statement_id   = "AllowAlarm${title(each.key)}"
  function_name  = aws_lambda_function.killswitch.function_name
  action         = "lambda:InvokeFunction"
  principal      = "lambda.alarms.cloudwatch.amazonaws.com"
  source_arn     = each.value.arn
  source_account = var.account_id
}
