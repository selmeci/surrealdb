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

variable "service_name" {
  description = "The name of service."
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
  type        = string
  description = "Run DB in strict mode"
}

variable "dynamodb_table_arn" {
  type        = string
  description = "ARN of DynamoDB table"
}

# server

variable "server_cpu" {
  type        = number
  description = "CPU setting for SurrealDB server task in Fargate"
}

variable "server_memory" {
  type        = number
  description = "Memory setting for SurrealDB server task in Fargate"
}

variable "server_max_capacity" {
  type        = number
  description = "Limit for horizontal scaling of SurrealDB server"
}

variable "shards" {
  type        = number
  description = "Number of shards used in DynamoDB"
}

variable "domain" {
  description = "Domain name for the hosted zone"
  type        = string
}
