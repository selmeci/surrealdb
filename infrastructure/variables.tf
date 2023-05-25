
variable "stage" {
  description = "Deployment stage."
  type        = string
  default     = "prod"
}

variable "region" {
  description = "AWS region for all resources."
  type        = string
  default     = "eu-central-1"
}

variable "table_name" {
  description = "The name of DynamoDB table."
  type        = string
  default     = "surrealdb"
}

variable "lambda_name" {
  description = "The name of Lambda function."
  type        = string
  default     = "surrealdb"
}

variable "log_retention_in_days" {
  type    = number
  default = 30
}

variable "log_level" {
  type    = string
  default = "trace"
}

variable "user" {
  type        = string
  default     = "root"
  description = "Username for root"
}

variable "pass" {
  type        = string
  default     = "pass"
  description = "Password for root"
}

variable "strict" {
  type        = bool
  default     = false
  description = "Run DB in strict mode"
}
