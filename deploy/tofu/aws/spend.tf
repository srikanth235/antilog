# The account's $50 monthly budget `antilog-dev` already exists and is read here as data,
# never recreated. Model keys are the customer's and the proxy meters spend, so there are
# no model-spend alarms here.

data "aws_budgets_budget" "dev" {
  name = var.budget_name
}
