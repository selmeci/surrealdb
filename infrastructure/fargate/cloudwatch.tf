# Set up CloudWatch group and log stream and retain logs for 30 days
resource "aws_cloudwatch_log_group" "surrealdb" {
  name              = "${local.path}/ecs"
  retention_in_days = 30
}

resource "aws_cloudwatch_log_stream" "surrealdb" {
  name           = "surrealdb"
  log_group_name = aws_cloudwatch_log_group.surrealdb.name
}
