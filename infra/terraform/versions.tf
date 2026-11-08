terraform {
  required_version = ">= 1.17.0, < 2.0.0"

  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 6.0"
    }
  }

  # State stays local until a shared backend is chosen. When one is needed,
  # replace this with an S3 backend (bucket + DynamoDB/S3 lock) created out of band.
  backend "local" {
    path = "terraform.tfstate"
  }
}
