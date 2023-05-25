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
      version = "~> 4.67"
    }
    null = {
      source  = "hashicorp/null"
      version = "3.2.1"
    }
  }

  required_version = "~> 1.4"
}

provider "aws" {
  region = var.region
}

data "aws_caller_identity" "current" {}
