resource "aws_dynamodb_table" "surrealdb" {
  name             = "${var.table_name}-${var.stage}"
  billing_mode     = "PAY_PER_REQUEST"
  stream_enabled   = true
  stream_view_type = "NEW_AND_OLD_IMAGES"
  hash_key         = "pk"


  point_in_time_recovery {
    enabled = true
  }

  attribute {
    name = "pk"
    type = "B"
  }

  attribute {
    name = "gsi1pk"
    type = "N"
  }

  ttl {
    attribute_name = "ttl"
    enabled        = true
  }

  global_secondary_index {
    name               = "GSI1"
    hash_key           = "gsi1pk"
    range_key          = "pk"
    projection_type    = "KEYS_ONLY"
  }

  tags = {
    Name        = var.lambda_name
    Environment = var.stage
  }
}
