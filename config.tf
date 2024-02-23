terraform {
  required_providers {
    fastly = {
      source = "fastly/fastly"
      version = "5.5.0"
    }
    aws = {
      source = "hashicorp/aws"
      version = "5.25.0"
    }
  }
  backend "s3" {
    bucket = "mt-entrypoint-state-files"
    key = "state"
    region = "eu-west-2"
  }
}

provider "fastly" {}
provider "aws" {}
