variable "mode" {
  description = "The mode for Terraform"
  default     = "fargate"
  type        = string

  validation {
    condition     = contains(["lambda", "fargate"], var.mode)
    error_message = "Invalid mode value. Allowed values are 'lambda' or 'fargate'."
  }
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

variable "name" {
  description = "The name of runtime service."
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

  validation {
    condition     = contains(["error", "warn", "info", "debug", "trace", "full"], var.log_level)
    error_message = "Invalid log level value. Allowed values are error, warn, info, debug, trace, full."
  }
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

variable "shards" {
  type        = number
  default     = 1
  description = "Number of shards used in DynamoDB"
}

variable "server_cpu" {
  type        = number
  description = "CPU setting for SurrealDB server task in Fargate"
  default     = 256
}

variable "server_memory" {
  type        = number
  description = "Memory setting for SurrealDB server task in Fargate"
  default     = 512
}

variable "server_max_capacity" {
  type        = number
  description = "Limit for horizontal scaling of SurrealDB server"
  default     = 8
}

variable "domain" {
  description = "Domain name for the hosted zone. Optional. If defined Route53 records and ACM certificate are created."
  type        = string
  default     = "selma-solutions.com"
}
