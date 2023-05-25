output "api_https_invoke_url" {
  value = "${aws_apigatewayv2_stage.stage.invoke_url}/"
}

output "lambda_invoke_url" {
  value = aws_lambda_function_url.surrealdb.function_url
}
