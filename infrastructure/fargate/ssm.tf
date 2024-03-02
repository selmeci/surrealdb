resource "aws_ssm_parameter" "root_password" {
  name        = "${local.path}/db/password/root"
  description = "Root password"
  type        = "SecureString"
  value       = var.pass
}
