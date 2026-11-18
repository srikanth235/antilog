# Agent sessions and judge sandboxes run on AgentCore, never on Lambda. Two runtimes with
# two roles: the agent role never reads the judge's credential. Both stay off until their
# images exist (agent_image_uri, judge_image_uri); an idle runtime costs nothing.

resource "aws_ecr_repository" "agent" {
  name                 = "${local.prefix}/agent"
  image_tag_mutability = "IMMUTABLE"

  image_scanning_configuration {
    scan_on_push = true
  }
}

resource "aws_ecr_repository" "judge" {
  name                 = "${local.prefix}/judge"
  image_tag_mutability = "IMMUTABLE"

  image_scanning_configuration {
    scan_on_push = true
  }
}

resource "aws_ecr_lifecycle_policy" "keep_twenty" {
  for_each   = { agent = aws_ecr_repository.agent.name, judge = aws_ecr_repository.judge.name }
  repository = each.value

  policy = jsonencode({
    rules = [{
      rulePriority = 1
      description  = "Keep the newest twenty images"
      selection    = { tagStatus = "any", countType = "imageCountMoreThan", countNumber = 20 }
      action       = { type = "expire" }
    }]
  })
}

data "aws_iam_policy_document" "runtime_trust" {
  statement {
    actions = ["sts:AssumeRole"]
    principals {
      type        = "Service"
      identifiers = ["bedrock-agentcore.amazonaws.com"]
    }
    condition {
      test     = "StringEquals"
      variable = "aws:SourceAccount"
      values   = [var.account_id]
    }
  }
}

resource "aws_iam_role" "agent_runtime" {
  name                 = "${local.prefix}-agent-runtime"
  description          = "Agent sessions: the two cheap models and the agent image"
  permissions_boundary = var.permissions_boundary_arn
  assume_role_policy   = data.aws_iam_policy_document.runtime_trust.json
}

resource "aws_iam_role" "judge_runtime" {
  name                 = "${local.prefix}-judge-runtime"
  description          = "Judge sandboxes: the judge image and the holdout credential"
  permissions_boundary = var.permissions_boundary_arn
  assume_role_policy   = data.aws_iam_policy_document.runtime_trust.json
}

resource "aws_iam_role_policy" "agent_runtime" {
  name = "agent-runtime"
  role = aws_iam_role.agent_runtime.id

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Sid      = "OnlyTheTwoCheapModels"
        Effect   = "Allow"
        Action   = ["bedrock:InvokeModel", "bedrock:InvokeModelWithResponseStream"]
        Resource = local.model_arns
      },
      {
        Sid      = "PullItsImage"
        Effect   = "Allow"
        Action   = ["ecr:BatchGetImage", "ecr:GetDownloadUrlForLayer"]
        Resource = aws_ecr_repository.agent.arn
      },
      {
        # GetAuthorizationToken has no resource scope; it only yields a registry login.
        Sid      = "RegistryLogin"
        Effect   = "Allow"
        Action   = "ecr:GetAuthorizationToken"
        Resource = "*"
      },
      {
        Sid      = "Logs"
        Effect   = "Allow"
        Action   = ["logs:CreateLogGroup", "logs:CreateLogStream", "logs:PutLogEvents"]
        Resource = "arn:aws:logs:${var.region}:${var.account_id}:log-group:/aws/bedrock-agentcore/runtimes/*"
      },
    ]
  })
}

resource "aws_iam_role_policy" "judge_runtime" {
  name = "judge-runtime"
  role = aws_iam_role.judge_runtime.id

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Sid      = "PullItsImage"
        Effect   = "Allow"
        Action   = ["ecr:BatchGetImage", "ecr:GetDownloadUrlForLayer"]
        Resource = aws_ecr_repository.judge.arn
      },
      {
        Sid      = "RegistryLogin"
        Effect   = "Allow"
        Action   = "ecr:GetAuthorizationToken"
        Resource = "*"
      },
      {
        Sid      = "TheHoldoutCredential"
        Effect   = "Allow"
        Action   = ["secretsmanager:GetSecretValue"]
        Resource = aws_secretsmanager_secret.judge_credential.arn
      },
      {
        Sid      = "Logs"
        Effect   = "Allow"
        Action   = ["logs:CreateLogGroup", "logs:CreateLogStream", "logs:PutLogEvents"]
        Resource = "arn:aws:logs:${var.region}:${var.account_id}:log-group:/aws/bedrock-agentcore/runtimes/*"
      },
    ]
  })
}

resource "aws_bedrockagentcore_agent_runtime" "agent" {
  count = var.agent_image_uri == "" ? 0 : 1

  agent_runtime_name = replace("${local.prefix}_agent", "-", "_") # runtime names allow letters, digits and underscores
  description        = "Antilog agent sessions"
  role_arn           = aws_iam_role.agent_runtime.arn

  agent_runtime_artifact {
    container_configuration {
      container_uri = var.agent_image_uri
    }
  }

  network_configuration {
    network_mode = "PUBLIC"
  }

  protocol_configuration {
    server_protocol = "HTTP"
  }

  environment_variables = {
    AWS_REGION = var.region
  }

  depends_on = [aws_iam_role_policy.agent_runtime]
}

resource "aws_bedrockagentcore_agent_runtime" "judge" {
  count = var.judge_image_uri == "" ? 0 : 1

  agent_runtime_name = replace("${local.prefix}_judge", "-", "_")
  description        = "Antilog judge sandboxes, one fresh session per measurement"
  role_arn           = aws_iam_role.judge_runtime.arn

  agent_runtime_artifact {
    container_configuration {
      container_uri = var.judge_image_uri
    }
  }

  network_configuration {
    network_mode = "PUBLIC"
  }

  protocol_configuration {
    server_protocol = "HTTP"
  }

  depends_on = [aws_iam_role_policy.judge_runtime]
}
