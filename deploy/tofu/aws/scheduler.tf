# EventBridge Scheduler is the core's timer: one one-off schedule per due job, in this
# group. A schedule invokes the server function and deletes itself after it fires.

resource "aws_scheduler_schedule_group" "wake" {
  name = local.prefix
}

resource "aws_iam_role" "scheduler" {
  name                 = "${local.prefix}-scheduler"
  description          = "Scheduler invokes the server function with this role"
  permissions_boundary = var.permissions_boundary_arn

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Effect    = "Allow"
      Principal = { Service = "scheduler.amazonaws.com" }
      Action    = "sts:AssumeRole"
      Condition = {
        StringEquals = { "aws:SourceAccount" = var.account_id }
        ArnLike      = { "aws:SourceArn" = "arn:aws:scheduler:${var.region}:${var.account_id}:schedule/${aws_scheduler_schedule_group.wake.name}/*" }
      }
    }]
  })
}

resource "aws_iam_role_policy" "scheduler" {
  name = "invoke-server"
  role = aws_iam_role.scheduler.id

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Effect   = "Allow"
      Action   = "lambda:InvokeFunction"
      Resource = aws_lambda_function.server.arn
    }]
  })
}
