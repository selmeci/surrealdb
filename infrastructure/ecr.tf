locals {
  root_dir        = "${path.module}/.."
  account_id      = data.aws_caller_identity.current.account_id
  build_args      = "--build-arg binary=surrealdb --build-arg log_level=${var.log_level}"
  repository_name = "${var.lambda_name}-${var.stage}"
  tag             = md5(sha256(join("", [for f in fileset("${local.root_dir}/aws_lambda/src", "**") : filesha256("${local.root_dir}/aws_lambda/src/${f}")])))
}


resource "aws_ecr_repository" "lambda_repository" {
  name         = local.repository_name
  force_delete = true
}

resource "null_resource" "lambda_ecr_image_builder" {
  triggers = {
    docker_file = filesha256("${local.root_dir}/aws_lambda/Dockerfile")
    cargo_file  = filesha256("${local.root_dir}/aws_lambda/Cargo.toml")
    src_dir     = local.tag
  }

  provisioner "local-exec" {
    working_dir = local.root_dir
    interpreter = ["/bin/bash", "-c"]
    command     = <<-EOT
      aws ecr get-login-password --region ${var.region} | docker login --username AWS --password-stdin ${local.account_id}.dkr.ecr.${var.region}.amazonaws.com
      docker image build -f aws_lambda/Dockerfile -t ${aws_ecr_repository.lambda_repository.repository_url}:${local.tag} ${local.build_args} .
      docker push ${aws_ecr_repository.lambda_repository.repository_url}:${local.tag}
    EOT
  }
}

data "aws_ecr_image" "lambda_image" {
  depends_on = [
    null_resource.lambda_ecr_image_builder
  ]

  repository_name = local.repository_name
  image_tag       = local.tag
}
