terraform {
  backend "s3" {
    # TODO specify your bucket
    bucket = ""
    key    = "tf/surrealdb.tf"
	# TODO your region
    region = "eu-central-1"
  }
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }
    null = {
      source  = "hashicorp/null"
      version = "3.2.1"
    }
  }

  required_version = "~> 1.5"
}

provider "aws" {
  region = var.region
}

data "aws_caller_identity" "current" {}

locals {
  stage = terraform.workspace
}

module "lambda" {
  source = "./lambda"
  count  = var.mode == "lambda" ? 1 : 0

  account_id            = data.aws_caller_identity.current.account_id
  domain                = var.domain
  dynamodb_table_arn    = aws_dynamodb_table.surrealdb.arn
  lambda_name           = var.name
  log_level             = var.log_level
  log_retention_in_days = var.log_retention_in_days
  pass                  = var.pass
  region                = var.region
  shards                = var.shards
  stage                 = local.stage
  strict                = var.strict
  table_name            = aws_dynamodb_table.surrealdb.name
  user                  = var.user
}

module "fargate" {
  source = "./fargate"
  count  = var.mode == "fargate" ? 1 : 0

  account_id            = data.aws_caller_identity.current.account_id
  domain                = var.domain
  dynamodb_table_arn    = aws_dynamodb_table.surrealdb.arn
  log_level             = var.log_level
  log_retention_in_days = var.log_retention_in_days
  pass                  = var.pass
  region                = var.region
  shards                = var.shards
  stage                 = local.stage
  strict                = var.strict
  table_name            = aws_dynamodb_table.surrealdb.name
  user                  = var.user
  server_cpu            = var.server_cpu
  server_max_capacity   = var.server_max_capacity
  server_memory         = var.server_memory
  service_name          = var.name
}
