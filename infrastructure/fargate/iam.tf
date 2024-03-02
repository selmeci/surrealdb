
data "aws_iam_policy_document" "assume_policy" {
  statement {
    actions = ["sts:AssumeRole"]

    principals {
      type        = "Service"
      identifiers = ["ecs-tasks.amazonaws.com"]
    }
  }
}

data "aws_iam_policy_document" "ssm_policy" {
  statement {
    actions = ["ssm:GetParameter", "ssm:GetParameters"]

    resources = ["*"]
  }
}

data "aws_iam_policy_document" "dynamodb_policy" {
  statement {
    actions = [
      "dynamodb:*",
    ]
    resources = ["${var.dynamodb_table_arn}*"]
  }
}

resource "aws_iam_policy" "dynamodb_policy" {
  name   = "dynamodb_${var.service_name}_${var.stage}"
  path   = "${local.path}/"
  policy = data.aws_iam_policy_document.dynamodb_policy.json
}

resource "aws_iam_policy" "ssm_policy" {
  name   = "ssm_${var.service_name}_${var.stage}"
  path   = "${local.path}/"
  policy = data.aws_iam_policy_document.ssm_policy.json
}

resource "aws_iam_role" "surrealdb" {
  name               = "execution_role_${var.service_name}_${var.stage}"
  path               = "${local.path}/"
  assume_role_policy = data.aws_iam_policy_document.assume_policy.json
}


resource "aws_iam_role_policy_attachment" "dynamodb_policy" {
  role       = aws_iam_role.surrealdb.name
  policy_arn = aws_iam_policy.dynamodb_policy.arn
}

resource "aws_iam_role_policy_attachment" "ssm_policy" {
  role       = aws_iam_role.surrealdb.name
  policy_arn = aws_iam_policy.ssm_policy.arn
}

# ECS task execution role policy attachment
resource "aws_iam_role_policy_attachment" "ecs_task_execution_role" {
  role       = aws_iam_role.surrealdb.name
  policy_arn = "arn:aws:iam::aws:policy/service-role/AmazonECSTaskExecutionRolePolicy"
}
