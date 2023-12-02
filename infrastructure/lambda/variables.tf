variable "account_id" {
  type        = string
  description = "AWS account id"
}

variable "stage" {
  description = "Deployment stage."
  type        = string
}

variable "region" {
  description = "AWS region for all resources."
  type        = string
}

variable "table_name" {
  description = "The name of DynamoDB table."
  type        = string
}

variable "lambda_name" {
  description = "The name of Lambda function."
  type        = string
}

variable "log_retention_in_days" {
  type = number
}

variable "log_level" {
  type = string
}

variable "user" {
  type        = string
  description = "Username for root"
}

variable "pass" {
  type        = string
  description = "Password for root"
}

variable "strict" {
  type        = bool
  description = "Run DB in strict mode"
}

variable "dynamodb_table_arn" {
  type        = string
  description = "ARN of DynamoDB table"
}

variable "shards" {
  type        = number
  description = "Number of shards used in DynamoDB"
}

variable "domain" {
  description = "Domain name for the hosted zone"
  type        = string
}
