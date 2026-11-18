terraform {
  required_version = ">= 1.10"

  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 6.73"
    }
    random = {
      source  = "hashicorp/random"
      version = "~> 3.9"
    }
    archive = {
      source  = "hashicorp/archive"
      version = "~> 2.8"
    }
  }

  # State lives in S3 with its own lock file (no DynamoDB). The bucket comes from
  # backend.hcl, created once by ./bootstrap:  tofu init -backend-config=backend.hcl
  backend "s3" {}
}

provider "aws" {
  region = var.region

  # Plans against another account fail instead of changing it.
  allowed_account_ids = [var.account_id]

  default_tags {
    tags = local.tags
  }
}
