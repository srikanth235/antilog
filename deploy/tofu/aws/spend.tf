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
# Deny-all on the antilog-* roles it locks and the server's concurrency to 0. It cannot lock
# itself or the CI role that releases it.

data "archive_file" "killswitch" {
  type        = "zip"
  source_file = "${path.module}/killswitch/handler.py"
  output_path = "${path.module}/.build/killswitch.zip"
}

resource "aws_cloudwatch_log_group" "killswitch" {
  name              = "/aws/lambda/${local.killswitch}"
  retention_in_days = 90
}

# The role is the owner's, outside antilog-*, so the boundary does not cap it and its own
# deny-all can never lock it. CI passes it to the function (the owner allows that).
data "aws_iam_role" "killswitch" {
  name = var.killswitch_role_name
}

resource "aws_lambda_function" "killswitch" {
  function_name = local.killswitch
  description   = "Spend kill switch: deny-all on antilog roles, concurrency 0"
  role          = data.aws_iam_role.killswitch.arn

  runtime       = "python3.13"
  architectures = ["arm64"]
  handler       = "handler.handler"
  timeout       = 120
  memory_size   = 128

  filename         = data.archive_file.killswitch.output_path
  source_code_hash = data.archive_file.killswitch.output_base64sha256

  environment {
    variables = {
      LOCKED_ROLES = join(",", local.locked_roles)
      SERVER       = local.server_function
    }
  }

  logging_config {
    log_format = "Text"
    log_group  = aws_cloudwatch_log_group.killswitch.name
  }
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
