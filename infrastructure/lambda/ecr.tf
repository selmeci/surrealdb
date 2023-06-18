locals {
  root_dir        = "${path.module}/.."
  repository_name = "${var.lambda_name}-${var.stage}"
  tag             = "1.0.0-beta.9-lambda-arm64"
  public_image    = "public.ecr.aws/i1b1y6t9/surrealdb:${local.tag}"
  ecr_image       = "${aws_ecr_repository.repository.repository_url}:${local.tag}"
}

resource "aws_ecr_repository" "repository" {
  name         = local.repository_name
  force_delete = true
}

resource "null_resource" "ecr_image_builder" {
  triggers = {
    tag = local.tag
  }

  provisioner "local-exec" {
    working_dir = local.root_dir
    interpreter = ["/bin/bash", "-c"]
    command     = <<-EOT
      aws ecr get-login-password --region ${var.region} | docker login --username AWS --password-stdin ${var.account_id}.dkr.ecr.${var.region}.amazonaws.com
      docker pull ${local.public_image}
      docker tag ${local.public_image} ${local.ecr_image}
      docker push ${local.ecr_image}
    EOT
  }
}

data "aws_ecr_image" "lambda_image" {
  depends_on = [
    null_resource.ecr_image_builder
  ]

  repository_name = local.repository_name
  image_tag       = local.tag
}
