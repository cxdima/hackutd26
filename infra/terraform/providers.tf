provider "aws" {
  region = var.aws_region

  # Credentials come from the standard AWS chain (AWS_PROFILE, SSO, env vars).
  # Never put access keys in this directory or in Git.
  default_tags {
    tags = {
      Project     = "firstring"
      Environment = var.environment
      ManagedBy   = "terraform"
    }
  }
}
