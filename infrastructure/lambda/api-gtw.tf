resource "aws_apigatewayv2_api" "surrealdb" {
  name          = "${var.lambda_name}-${var.stage}"
  protocol_type = "HTTP"

  cors_configuration {
    allow_origins = ["*"]
    allow_methods = ["*"]
    allow_headers = ["*"]
    max_age       = 0
  }
}

resource "aws_apigatewayv2_route" "route" {
  api_id    = aws_apigatewayv2_api.surrealdb.id
  route_key = "ANY /{proxy+}"
  target    = "integrations/${aws_apigatewayv2_integration.integration.id}"
}

resource "aws_apigatewayv2_integration" "integration" {
  api_id           = aws_apigatewayv2_api.surrealdb.id
  integration_type = "AWS_PROXY"
  integration_uri  = aws_lambda_function.surrealdb.invoke_arn

  payload_format_version = "2.0"

  request_parameters = {
    "overwrite:path" = "$request.path"
  }
}

resource "aws_lambda_permission" "apigw" {
  statement_id  = "AllowExecutionFromAPIGateway"
  action        = "lambda:InvokeFunction"
  function_name = aws_lambda_function.surrealdb.function_name
  principal     = "apigateway.amazonaws.com"
  source_arn    = "${aws_apigatewayv2_api.surrealdb.execution_arn}/*/*"
}

resource "aws_apigatewayv2_stage" "stage" {
  api_id      = aws_apigatewayv2_api.surrealdb.id
  name        = var.stage
  auto_deploy = true
}


