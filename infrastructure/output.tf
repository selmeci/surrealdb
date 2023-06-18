output "https_invoke_url" {
  value = length(module.lambda) != 0 ? module.lambda[0].api_https_invoke_url : "NOT AVAIABLE"
}

output "api_https_invoke_url" {
  value = length(module.lambda) != 0 ? module.lambda[0].api_https_invoke_url : "NOT AVAIABLE"
}

output "lambda_invoke_url" {
  value = length(module.lambda) != 0 ? module.lambda[0].lambda_invoke_url : "NOT AVAIABLE"
}

output "ws_invoke_url" {
  value = length(module.fargate) != 0 ? module.fargate[0].ws_invoke_url : "NOT AVAIABLE"
}

output "domain_invoke_url" {
  value = length(module.lambda) != 0 ? module.lambda[0].domain_https_invoke_url : module.fargate[0].domain_ws_invoke_url
}
