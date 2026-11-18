# Aurora Serverless v2 (PostgreSQL) at a minimum of 0 ACUs: it pauses when idle and the
# first request after a pause waits about 15 seconds. Idle cost is storage only.
# No RDS Proxy (it has an hourly fee): the function keeps one connection per instance.

data "aws_vpc" "default" {
  default = true
}

data "aws_subnets" "default" {
  filter {
    name   = "vpc-id"
    values = [data.aws_vpc.default.id]
  }
}

resource "aws_security_group" "database" {
  name        = "${local.prefix}-database"
  description = "PostgreSQL for Antilog; the function has no fixed address, see database_ingress_cidrs"
  vpc_id      = data.aws_vpc.default.id
}

resource "aws_vpc_security_group_ingress_rule" "postgres" {
  for_each = toset(var.database_ingress_cidrs)

  security_group_id = aws_security_group.database.id
  cidr_ipv4         = each.value
  from_port         = 5432
  to_port           = 5432
  ip_protocol       = "tcp"
  description       = "PostgreSQL over TLS"
}

resource "aws_db_subnet_group" "database" {
  name       = "${local.prefix}-database"
  subnet_ids = data.aws_subnets.default.ids
}

resource "aws_rds_cluster_parameter_group" "database" {
  name   = "${local.prefix}-aurora-postgresql18"
  family = "aurora-postgresql18"

  parameter {
    name  = "rds.force_ssl"
    value = "1"
  }
}

resource "random_password" "database" {
  length  = 40
  special = false # goes into a connection URL
}

resource "aws_rds_cluster" "database" {
  cluster_identifier = "${local.prefix}-${var.environment}"
  engine             = "aurora-postgresql"
  engine_mode        = "provisioned"
  engine_version     = var.database_engine_version

  database_name   = "antilog"
  master_username = "antilog"
  master_password = random_password.database.result

  db_subnet_group_name            = aws_db_subnet_group.database.name
  db_cluster_parameter_group_name = aws_rds_cluster_parameter_group.database.name
  vpc_security_group_ids          = [aws_security_group.database.id]

  storage_encrypted         = true
  backup_retention_period   = 7
  deletion_protection       = var.database_deletion_protection
  skip_final_snapshot       = false
  final_snapshot_identifier = "${local.prefix}-${var.environment}-final"
  apply_immediately         = true

  serverlessv2_scaling_configuration {
    min_capacity             = 0
    max_capacity             = var.database_max_acu
    seconds_until_auto_pause = var.database_seconds_until_auto_pause
  }

  lifecycle {
    ignore_changes = [engine_version] # minor upgrades are done by hand, in a plan of their own
  }
}

resource "aws_rds_cluster_instance" "database" {
  identifier         = "${local.prefix}-${var.environment}-1"
  cluster_identifier = aws_rds_cluster.database.id
  engine             = aws_rds_cluster.database.engine
  engine_version     = aws_rds_cluster.database.engine_version
  instance_class     = "db.serverless" # the CI policy refuses any other class

  publicly_accessible = true # no NAT gateway: a function outside a VPC reaches it over TLS
}
