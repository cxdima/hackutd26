output "aws_region" {
  description = "Region the configuration is deployed to."
  value       = var.aws_region
}

output "name_servers" {
  description = "Route 53 name servers to enter at the domain registrar (GoDaddy)."
  value       = aws_route53_zone.site.name_servers
}

output "hosted_zone_id" {
  description = "Route 53 hosted zone id for the site domain."
  value       = aws_route53_zone.site.zone_id
}

output "bucket_name" {
  description = "Private S3 bucket holding the built site (used by infra/deploy-site.sh)."
  value       = aws_s3_bucket.site.bucket
}

output "distribution_id" {
  description = "CloudFront distribution id (used by infra/deploy-site.sh for invalidations)."
  value       = aws_cloudfront_distribution.site.id
}

output "cloudfront_domain_name" {
  description = "Default CloudFront hostname; serves the site in both phases."
  value       = aws_cloudfront_distribution.site.domain_name
}

output "certificate_arn" {
  description = "ACM certificate for the apex and www hostnames (us-east-1)."
  value       = aws_acm_certificate.site.arn
}

output "custom_domain_enabled" {
  description = "Whether phase 2 (custom hostnames on CloudFront) is applied."
  value       = var.enable_custom_domain
}

output "site_url" {
  description = "Public URL of the site for the current phase."
  value       = var.enable_custom_domain ? "https://${var.domain_name}" : "https://${aws_cloudfront_distribution.site.domain_name}"
}
