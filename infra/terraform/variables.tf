variable "aws_region" {
  description = "AWS region. The static site lives in us-east-1 (CloudFront certificates must be issued there)."
  type        = string
  default     = "us-east-1"
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

variable "domain_name" {
  description = "Apex domain served by the static site. www.<domain> is added as a second hostname."
  type        = string
  default     = "firstring.life"
}

variable "enable_custom_domain" {
  description = <<-EOT
    Phase 2 switch. false (phase 1): the site is served only at its *.cloudfront.net address.
    true (phase 2, after the registrar points the domain at the Route 53 name servers): wait for
    the ACM certificate, attach the apex and www hostnames to CloudFront and create alias records.
  EOT
  type        = bool
  default     = false
}

variable "dmarc_record" {
  description = "_dmarc TXT value copied from the previous GoDaddy DNS zone."
  type        = string
  default     = "v=DMARC1; p=quarantine; adkim=r; aspf=r; rua=mailto:dmarc_rua@onsecureserver.net;"
}
