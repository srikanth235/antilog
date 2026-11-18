locals {
  tags = {
    Project     = "antilog"
    Environment = var.environment
    ManagedBy   = "opentofu"
    Repository  = "srikanth235/antilog"
  }

  prefix = var.name

  # The only two models the account may call (the boundary and the CI policy say the same).
  # Prices are dollars per million tokens. Change them here and in the boundary together.
  models = {
    "zai.glm-4.7-flash"             = { input = 0.07, output = 0.40 }
    "qwen.qwen3-coder-30b-a3b-v1:0" = { input = 0.15, output = 0.60 }
  }
  agent_model    = "zai.glm-4.7-flash"
  fallback_model = "qwen.qwen3-coder-30b-a3b-v1:0"
  model_arns     = [for id in keys(local.models) : "arn:aws:bedrock:${var.region}::foundation-model/${id}"]

  server_function = "${local.prefix}-server"
  killswitch      = "${local.prefix}-killswitch"

  # Roles the kill switch locks. Not its own, and not the CI role that must undo it.
  locked_roles = [
    aws_iam_role.server.name,
    aws_iam_role.scheduler.name,
    aws_iam_role.agent_runtime.name,
    aws_iam_role.judge_runtime.name,
  ]
}
