locals {
  project_name = "${var.lambda_name}-${var.stage}"
}

resource "aws_iam_role" "lambda_execution_role" {
  name = "${var.lambda_name}-iam-role-${var.region}-${var.stage}"
  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Sid    = ""
        Effect = "Allow"
        Action = "sts:AssumeRole"
        Principal = {
          Service = "lambda.amazonaws.com"
        }
      }
    ]
  })
}

resource "aws_iam_role_policy_attachment" "lambda_execution_policy" {
  role       = aws_iam_role.lambda_execution_role.name
  policy_arn = "arn:aws:iam::aws:policy/service-role/AWSLambdaBasicExecutionRole"
}

resource "aws_lambda_function" "surrealdb" {
  depends_on = [
    aws_iam_role_policy_attachment.lambda_logs,
    aws_cloudwatch_log_group.lambda,
    null_resource.ecr_image_builder,
  ]

  architectures = ["arm64"]

  function_name = "${var.lambda_name}-${var.stage}"
  image_uri     = local.ecr_image
  package_type  = "Image"
  memory_size   = 512
  timeout       = 30

  environment {
    variables = {
      "RUST_LOG"       = "aws_lambda=${var.log_level},surreal=${var.log_level},surrealdb=${var.log_level},surrealdb::net=${var.log_level}",
      "LOG_LVL"        = var.log_level,
      "RUST_MIN_STACK" = "8388608",
      "TABLE"          = var.table_name,
      "SHARDS"         = var.shards,
      "USER"           = var.user,
      "PASS"           = var.pass,
      "STRICT"         = var.strict,
      "STAGE"          = var.stage
    }
  }

  role = aws_iam_role.lambda_execution_role.arn
}

data "aws_iam_policy_document" "db_policy" {
  statement {
    effect = "Allow"
    actions = [
      "dynamodb:*",
    ]
    resources = ["${var.dynamodb_table_arn}*"]
  }
}

resource "aws_iam_role_policy" "db" {
  name   = "${var.lambda_name}-lambda-db-${var.stage}"
  role   = aws_iam_role.lambda_execution_role.id
  policy = data.aws_iam_policy_document.db_policy.json
}

data "aws_iam_policy_document" "lambda_logging" {
  statement {
    effect = "Allow"
    actions = [
      "logs:CreateLogGroup",
      "logs:CreateLogStream",
      "logs:PutLogEvents",
    ]
    resources = ["arn:aws:logs:*:*:*"]
  }
}

resource "aws_iam_policy" "lambda_logging" {
  name        = "${var.lambda_name}-lambda-logging-${var.stage}"
  path        = "/"
  description = "IAM policy for logging from a lambda"
  policy      = data.aws_iam_policy_document.lambda_logging.json
}

resource "aws_iam_role_policy_attachment" "lambda_logs" {
  role       = aws_iam_role.lambda_execution_role.name
  policy_arn = aws_iam_policy.lambda_logging.arn
}

resource "aws_lambda_function_url" "surrealdb" {
  function_name      = aws_lambda_function.surrealdb.function_name
  authorization_type = "NONE"

  cors {
    allow_credentials = true
    allow_origins     = ["*"]
    allow_methods     = ["*"]
    allow_headers     = ["*"]
    expose_headers    = ["*"]
    max_age           = 0
  }
}

resource "aws_cloudwatch_log_group" "lambda" {
  name              = "/aws/lambda/${var.lambda_name}/${var.stage}"
  retention_in_days = var.log_retention_in_days
}
