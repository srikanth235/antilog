# Secrets Manager holds every secret; the function gets its values in its environment at
# deploy time and never calls Secrets Manager itself. No value is written in code: the
# database password and the receipt key are generated here (they are in the encrypted
# state), and anything an outside system issues is put in its entry by the owner.

resource "aws_secretsmanager_secret" "database_url" {
  name                    = "${local.prefix}/database-url"
  description             = "The server's connection string"
  recovery_window_in_days = 7
}

resource "aws_secretsmanager_secret_version" "database_url" {
  secret_id = aws_secretsmanager_secret.database_url.id
  secret_string = format(
    "postgres://%s:%s@%s:5432/%s?sslmode=require",
    aws_rds_cluster.database.master_username,
    random_password.database.result,
    aws_rds_cluster.database.endpoint,
    aws_rds_cluster.database.database_name,
  )
}

# The ed25519 seed that signs receipts: 32 random bytes as 64 hex characters.
resource "random_id" "receipt_key" {
  byte_length = 32
}

resource "aws_secretsmanager_secret" "receipt_key" {
  name                    = "${local.prefix}/receipt-key"
  description             = "Signs receipts (ed25519 seed, hex)"
  recovery_window_in_days = 7
}

resource "aws_secretsmanager_secret_version" "receipt_key" {
  secret_id     = aws_secretsmanager_secret.receipt_key.id
  secret_string = random_id.receipt_key.hex
}

# Issued by GitHub: the owner puts the value in this entry, then sets signin_secrets_populated.
resource "aws_secretsmanager_secret" "github_client_secret" {
  name                    = "${local.prefix}/github-client-secret"
  description             = "GitHub OAuth app client secret (set by the owner)"
  recovery_window_in_days = 7
}

data "aws_secretsmanager_secret_version" "github_client_secret" {
  count     = var.signin_secrets_populated ? 1 : 0
  secret_id = aws_secretsmanager_secret.github_client_secret.id
}

# The judge's own credential for the holdout, which no agent role can read.
resource "aws_secretsmanager_secret" "judge_credential" {
  name                    = "${local.prefix}/judge-credential"
  description             = "Read access to the holdout, for the judge runtime only (set by the owner)"
  recovery_window_in_days = 7
}
