output "api_https_invoke_url" {
  value = "${aws_apigatewayv2_stage.stage.invoke_url}/"
}

output "lambda_invoke_url" {
  value = aws_lambda_function_url.surrealdb.function_url
}

output "domain_https_invoke_url" {
  value = var.domain != "" ? "https://${local.api_gtw_domain_name}" : "NOT AVAIABLE"
}
