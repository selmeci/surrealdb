output "ws_invoke_url" {
  value = "wss://${aws_lb.surrealdb.dns_name}"
}

output "domain_ws_invoke_url" {
  value = var.domain != "" ? "wss://${local.alb_domain_name}" : "NOT AVAIABLE"
}
