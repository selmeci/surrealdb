resource "aws_dynamodb_table" "surrealdb" {
  name             = "${var.table_name}-${var.stage}"
  billing_mode     = "PAY_PER_REQUEST"
  stream_enabled   = true
  stream_view_type = "NEW_AND_OLD_IMAGES"
  hash_key         = "pk"
  range_key        = "key"


  point_in_time_recovery {
    enabled = true
  }

  attribute {
    name = "pk"
    type = "B"
  }

  attribute {
    name = "key"
    type = "B"
  }

  attribute {
    name = "bucket"
    type = "B"
  }

  ttl {
    attribute_name = "ttl"
    enabled        = true
  }

  global_secondary_index {
    name            = "GSI1"
    hash_key        = "bucket"
    range_key       = "key"
    projection_type = "ALL"
  }

  tags = {
    Name        = "SurrealDb"
    Environment = var.stage
  }
}
