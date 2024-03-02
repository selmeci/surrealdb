
resource "aws_ecs_cluster" "surrealdb" {
  name = local.project_name

  setting {
    name  = "containerInsights"
    value = "enabled"
  }

  configuration {
    execute_command_configuration {
      logging = "OVERRIDE"

      log_configuration {
        cloud_watch_log_group_name = aws_cloudwatch_log_group.surrealdb.name
      }
    }
  }
}

resource "aws_ecs_cluster_capacity_providers" "surrealdb" {
  cluster_name = aws_ecs_cluster.surrealdb.name

  capacity_providers = ["FARGATE", "FARGATE_SPOT"]

  default_capacity_provider_strategy {
    base              = 1
    weight            = 10
    capacity_provider = "FARGATE"
  }
}

locals {
  ecr_environment = [
    { name : "RUST_MIN_STACK", value : "8388608" },
    { name : "SURREAL_PATH", value : "dynamodb://${var.table_name}?shards=${var.shards}" },
    { name : "SURREAL_CAPS_ALLOW_ALL", value : "true" },
    { name : "SURREAL_AUTH", value : "true" },
    { name : "SURREAL_USER", value : "root" },
    { name : "SURREAL_STRICT", value : var.strict },
    { name : "SURREAL_LOG", value : var.log_level }
  ]
  ecr_secrets = [
    { name : "SURREAL_PASS", valueFrom : aws_ssm_parameter.root_password.arn },
  ]
  surrealdb_container_definition = [
    {
      command : ["start"],
      name : "surrealdb",
      image : local.ecr_image,
      cpu : var.server_cpu,
      memory : var.server_memory,
      portMappings : [
        {
          containerPort : 8000,
          protocol : "tcp"
        }
      ],
      healthCheck : {
        command : [
          "CMD-SHELL",
          "curl -f http://127.0.0.1:8000${local.health_path} || exit 1"
        ],
        interval : 30,
        timeout : 5,
        retries : 3,
        startPeriod : 60
      },
      environment : local.ecr_environment,
      secrets : local.ecr_secrets,
      logConfiguration : {
        logDriver : "awslogs",
        options : {
          "awslogs-group" : aws_cloudwatch_log_group.surrealdb.name,
          "awslogs-region" : var.region,
          "awslogs-stream-prefix" : "surrealdb"
        }
      },
      essential : true
    }
  ]
}

resource "aws_ecs_task_definition" "surrealdb" {
  depends_on = [null_resource.ecr_image_builder]

  family                   = local.project_name
  requires_compatibilities = ["FARGATE"]
  network_mode             = "awsvpc"
  cpu                      = var.server_cpu
  memory                   = var.server_memory
  runtime_platform {
    operating_system_family = "LINUX"
  }

  execution_role_arn = aws_iam_role.surrealdb.arn
  task_role_arn      = aws_iam_role.surrealdb.arn

  container_definitions = jsonencode(local.surrealdb_container_definition)
}

resource "aws_ecs_service" "surrealdb" {
  depends_on = [aws_iam_role_policy_attachment.ecs_task_execution_role, null_resource.ecr_image_builder]

  name                              = "surrealdb"
  cluster                           = aws_ecs_cluster.surrealdb.id
  task_definition                   = aws_ecs_task_definition.surrealdb.arn
  desired_count                     = 1
  health_check_grace_period_seconds = 30
  wait_for_steady_state             = true

  capacity_provider_strategy {
    capacity_provider = "FARGATE"
    base              = 1
    weight            = 10
  }

  deployment_circuit_breaker {
    enable   = false
    rollback = false
  }

  deployment_controller {
    type = "ECS"
  }

  network_configuration {
    subnets          = aws_subnet.public.*.id
    security_groups  = [aws_security_group.ecs_tasks.id]
    assign_public_ip = true
  }

  load_balancer {
    target_group_arn = aws_lb_target_group.surrealdb.arn
    container_name   = "surrealdb"
    container_port   = 8000
  }
}
