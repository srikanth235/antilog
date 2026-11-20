locals {
  tags = {
    Project     = "antilog"
    Environment = var.environment
    ManagedBy   = "opentofu"
    Repository  = "srikanth235/antilog"
  }

  prefix = var.name

  server_function = "${local.prefix}-server"
}
