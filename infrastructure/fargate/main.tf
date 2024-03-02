locals {
  health_path  = "/health"
  port         = 8000
  path         = "/${var.service_name}/${var.stage}"
  project_name = "${var.service_name}-${var.stage}"
}
