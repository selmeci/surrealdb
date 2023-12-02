
resource "aws_lb" "surrealdb" {
  name               = local.project_name
  internal           = false
  load_balancer_type = "application"
  security_groups    = [aws_security_group.lb.id]
  subnets            = aws_subnet.public.*.id

  enable_deletion_protection = false
}

resource "aws_lb_target_group" "surrealdb" {
  name        = local.project_name
  target_type = "ip"
  port        = 8000
  protocol    = "HTTP"
  vpc_id      = aws_vpc.main.id

  health_check {
    path     = local.health_path
    port     = local.port
    protocol = "HTTP"
    interval = 30
  }
}

resource "aws_lb_listener" "http_surrealdb" {
  count = var.domain != "" ? 0 : 1

  load_balancer_arn = aws_lb.surrealdb.arn
  port              = 80
  protocol          = "HTTP"

  default_action {
    type             = "forward"
    target_group_arn = aws_lb_target_group.surrealdb.arn
  }
}

resource "aws_lb_listener" "https_listener" {
  count = var.domain != "" ? 1 : 0

  load_balancer_arn = aws_lb.surrealdb.arn
  port              = 443
  protocol          = "HTTPS"
  ssl_policy        = "ELBSecurityPolicy-2016-08"

  certificate_arn = aws_acm_certificate.ssl_certificate[0].arn

  default_action {
    type             = "forward"
    target_group_arn = aws_lb_target_group.surrealdb.arn
  }
}
