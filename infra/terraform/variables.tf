variable "aws_region" {
  description = "AWS region for FirstRing server functions."
  type        = string
  default     = "us-east-2"
}

variable "environment" {
  description = "Deployment environment name (for example: dev, demo)."
  type        = string
  default     = "dev"

  validation {
    condition     = can(regex("^[a-z][a-z0-9-]{1,20}$", var.environment))
    error_message = "Use a short lowercase name such as dev or demo."
  }
}
