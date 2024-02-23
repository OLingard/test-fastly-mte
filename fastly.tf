

locals {
  hosted_zone_id       = "Z0311841A4E5MH312TR4"
  base_domain          = "entrypoint.thgaltitude.com"
  workspace_hash       = sha256(terraform.workspace)
  env                  = substr(local.workspace_hash, 0, 8)
  service_domain       = "${local.env}.${local.base_domain}"
  deliver_domain       = "${local.env}-deliver.${local.base_domain}"
  deliver_domain2      = "${local.env}-deliver2.${local.base_domain}"
  resource_name_prefix = "test-mt-entrypoint-${local.env}"
}

resource "terraform_data" "mt-entrypoint" {
  triggers_replace = [
    filesha256("${path.module}/Cargo.toml"),
    filesha256("${path.module}/Cargo.lock"),
    filesha256("${path.module}/rust-toolchain.toml"),
    fileexists("${path.module}/pkg/entrypoint.tar.gz") ? "1" : uuid(),
    sha256(join("", [for f in fileset("${path.module}/src", "**") : filesha256("${path.module}/src/${f}")]))
  ]

  provisioner "local-exec" {
    working_dir = path.module
    interpreter = ["/bin/bash", "-c"]
    command     = "echo $PATH && fastly compute build"
  }
}

data "fastly_package_hash" "mt-entrypoint" {
  filename   = "${path.module}/pkg/entrypoint.tar.gz"
  depends_on = [terraform_data.mt-entrypoint]
}

resource "fastly_configstore" "mapper" {
  name          = "entrypoint-mapper"
  force_destroy = true
}

resource "fastly_kvstore" "routes" {
  name          = "${local.resource_name_prefix}-routes"
  force_destroy = true
}

resource "fastly_service_compute" "mt-entrypoint" {
  name = local.resource_name_prefix

  domain {
    name = local.service_domain
  }

  package {
    filename         = data.fastly_package_hash.mt-entrypoint.filename
    source_code_hash = data.fastly_package_hash.mt-entrypoint.hash
  }

  force_destroy = true

  resource_link {
    name        = "routes_kv_link"
    resource_id = fastly_kvstore.routes.id
  }

  resource_link {
    name        = "mapper_config_link"
    resource_id = fastly_configstore.mapper.id
  }
}

resource "fastly_service_vcl" "deliver-service" {
  name = "mt-entrypoint-test"

  domain {
    name = local.deliver_domain
  }

  domain {
    name = local.deliver_domain2
  }

  backend {
    name               = fastly_service_compute.mt-entrypoint.name
    address            = local.service_domain
    port               = 443
    use_ssl            = true
    connect_timeout    = 50
    first_byte_timeout = 10000
    ssl_cert_hostname  = local.service_domain
    ssl_sni_hostname   = local.service_domain
    override_host      = local.service_domain
  }

  force_destroy = true
}

resource "aws_route53_record" "mt-entrypoint" {
  zone_id = local.hosted_zone_id
  name    = local.service_domain
  type    = "CNAME"
  ttl     = 60
  records = [
    "n.sni.global.fastly.net"
  ]
}

resource "aws_route53_record" "deliver-mt-entrypoint" {
  zone_id = local.hosted_zone_id
  name    = local.deliver_domain
  type    = "CNAME"
  ttl     = 60
  records = [
    "n.sni.global.fastly.net"
  ]
}

resource "aws_route53_record" "deliver2-mt-entrypoint" {
  zone_id = local.hosted_zone_id
  name    = local.deliver_domain2
  type    = "CNAME"
  ttl     = 60
  records = [
    "n.sni.global.fastly.net"
  ]
}

resource "fastly_tls_subscription" "mt-entrypoint" {
  domains               = [local.service_domain]
  certificate_authority = "certainly"
  force_destroy         = true
}

resource "fastly_tls_subscription" "deliver-mt-entrypoint" {
  domains               = [local.deliver_domain]
  certificate_authority = "certainly"
  force_destroy         = true
}

resource "fastly_tls_subscription" "deliver2-mt-entrypoint" {
  domains               = [local.deliver_domain2]
  certificate_authority = "certainly"
  force_destroy         = true
}

resource "fastly_tls_subscription_validation" "mt-entrypoint" {
  subscription_id = fastly_tls_subscription.mt-entrypoint.id
}

resource "fastly_tls_subscription_validation" "deliver-mt-entrypoint" {
  subscription_id = fastly_tls_subscription.deliver-mt-entrypoint.id
}

resource "fastly_tls_subscription_validation" "deliver2-mt-entrypoint" {
  subscription_id = fastly_tls_subscription.deliver2-mt-entrypoint.id
}

output "fastly_service_endpoint" {
  value = local.service_domain
}
