# Static hosting for the Angular console on firstring.life.
#
# Phase 1 (enable_custom_domain = false): Route 53 zone, private S3 bucket, ACM certificate with
# its DNS validation records, and a CloudFront distribution on the default *.cloudfront.net
# certificate. The site is live at the CloudFront address immediately.
#
# Phase 2 (enable_custom_domain = true): after the registrar uses the Route 53 name servers, ACM
# can validate. Terraform waits for the certificate, attaches firstring.life and
# www.firstring.life to CloudFront and creates A/AAAA alias records.

locals {
  site_hostnames = [var.domain_name, "www.${var.domain_name}"]

  site_alias_records = var.enable_custom_domain ? {
    for pair in setproduct(local.site_hostnames, ["A", "AAAA"]) :
    "${pair[0]}-${pair[1]}" => { name = pair[0], type = pair[1] }
  } : {}
}

# ---------------------------------------------------------------- DNS zone

resource "aws_route53_zone" "site" {
  name    = var.domain_name
  comment = "FirstRing static site (managed by Terraform)"
}

# Copied from the previous GoDaddy zone. The parked A record and _domainconnect CNAME are
# deliberately not replicated.
resource "aws_route53_record" "dmarc" {
  zone_id = aws_route53_zone.site.zone_id
  name    = "_dmarc.${var.domain_name}"
  type    = "TXT"
  ttl     = 3600
  records = [var.dmarc_record]
}

# ---------------------------------------------------------------- S3 origin (private)

resource "aws_s3_bucket" "site" {
  bucket_prefix = "firstring-life-site-"

  # The bucket holds only rebuildable build output.
  force_destroy = true
}

resource "aws_s3_bucket_public_access_block" "site" {
  bucket = aws_s3_bucket.site.id

  block_public_acls       = true
  block_public_policy     = true
  ignore_public_acls      = true
  restrict_public_buckets = true
}

resource "aws_s3_bucket_ownership_controls" "site" {
  bucket = aws_s3_bucket.site.id

  rule {
    object_ownership = "BucketOwnerEnforced"
  }
}

resource "aws_s3_bucket_server_side_encryption_configuration" "site" {
  bucket = aws_s3_bucket.site.id

  rule {
    apply_server_side_encryption_by_default {
      sse_algorithm = "AES256"
    }
  }
}

# Only this distribution (via Origin Access Control) may read objects. Inline JSON rather than an
# aws_iam_policy_document data source, so phase 2's in-place distribution update does not show a
# spurious bucket-policy diff.
resource "aws_s3_bucket_policy" "site" {
  bucket = aws_s3_bucket.site.id
  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Sid       = "AllowCloudFrontOriginAccessControlRead"
      Effect    = "Allow"
      Principal = { Service = "cloudfront.amazonaws.com" }
      Action    = "s3:GetObject"
      Resource  = "${aws_s3_bucket.site.arn}/*"
      Condition = { StringEquals = { "AWS:SourceArn" = aws_cloudfront_distribution.site.arn } }
    }]
  })

  depends_on = [aws_s3_bucket_public_access_block.site]
}

# ---------------------------------------------------------------- Certificate (us-east-1)

# Created in phase 1 so ACM starts polling for its validation records right away; it is issued
# automatically once the domain's name servers point at the Route 53 zone. ACM gives up on a
# pending validation after 72 hours (recreate with `terraform apply -replace=aws_acm_certificate.site`).
resource "aws_acm_certificate" "site" {
  region                    = "us-east-1"
  domain_name               = var.domain_name
  subject_alternative_names = ["www.${var.domain_name}"]
  validation_method         = "DNS"

  lifecycle {
    create_before_destroy = true
  }
}

resource "aws_route53_record" "cert_validation" {
  for_each = {
    for dvo in aws_acm_certificate.site.domain_validation_options : dvo.domain_name => {
      name   = dvo.resource_record_name
      record = dvo.resource_record_value
      type   = dvo.resource_record_type
    }
  }

  allow_overwrite = true
  zone_id         = aws_route53_zone.site.zone_id
  name            = each.value.name
  type            = each.value.type
  ttl             = 60
  records         = [each.value.record]
}

resource "aws_acm_certificate_validation" "site" {
  count = var.enable_custom_domain ? 1 : 0

  region                  = "us-east-1"
  certificate_arn         = aws_acm_certificate.site.arn
  validation_record_fqdns = [for record in aws_route53_record.cert_validation : record.fqdn]

  timeouts {
    create = "30m"
  }
}

# ---------------------------------------------------------------- CloudFront

data "aws_cloudfront_cache_policy" "caching_optimized" {
  name = "Managed-CachingOptimized"
}

resource "aws_cloudfront_origin_access_control" "site" {
  name                              = "firstring-life-site"
  description                       = "FirstRing static site bucket"
  origin_access_control_origin_type = "s3"
  signing_behavior                  = "always"
  signing_protocol                  = "sigv4"
}

data "aws_cloudfront_cache_policy" "caching_disabled" {
  name = "Managed-CachingDisabled"
}

data "aws_cloudfront_origin_request_policy" "all_viewer_except_host" {
  name = "Managed-AllViewerExceptHostHeader"
}

locals {
  # The live API is optional: only when a tunnel host and a long edge secret are supplied.
  live_api_enabled = var.live_api_host != "" && length(var.edge_read_secret) >= 24
}

# Angular deep links (/console/present, /console/calls/:id, ...) are not S3 objects. A
# viewer-request function serves the SPA shell for extension-less paths on the S3 origin
# only, so API errors from the live origin keep their real status codes.
resource "aws_cloudfront_function" "spa" {
  name    = "firstring-spa-shell"
  runtime = "cloudfront-js-2.0"
  publish = true
  code    = <<-EOT
    function handler(event) {
      var request = event.request;
      var uri = request.uri;
      if (uri.lastIndexOf('.') <= uri.lastIndexOf('/')) {
        request.uri = '/index.html';
      }
      return request;
    }
  EOT
}

resource "aws_cloudfront_distribution" "site" {
  enabled             = true
  comment             = "FirstRing static site (${var.domain_name})"
  default_root_object = "index.html"
  price_class         = "PriceClass_100"
  http_version        = "http2and3"
  is_ipv6_enabled     = true
  aliases             = var.enable_custom_domain ? local.site_hostnames : []

  origin {
    origin_id                = "s3-site"
    domain_name              = aws_s3_bucket.site.bucket_regional_domain_name
    origin_access_control_id = aws_cloudfront_origin_access_control.site.id
  }

  # Read-only live data: GET/HEAD /api/* goes through the Cloudflare tunnel to the FirstRing
  # server on the presenting laptop. The server accepts this secret header only for GETs on
  # an allowlist of read-only paths; writes, recordings and voice cloning stay local.
  dynamic "origin" {
    for_each = local.live_api_enabled ? [var.live_api_host] : []
    content {
      origin_id   = "live-api"
      domain_name = origin.value
      custom_origin_config {
        http_port                = 80
        https_port               = 443
        origin_protocol_policy   = "https-only"
        origin_ssl_protocols     = ["TLSv1.2"]
        origin_read_timeout      = 60
        origin_keepalive_timeout = 60
      }
      custom_header {
        name  = "x-firstring-edge"
        value = var.edge_read_secret
      }
    }
  }

  dynamic "ordered_cache_behavior" {
    for_each = local.live_api_enabled ? ["/api/*"] : []
    content {
      path_pattern             = ordered_cache_behavior.value
      target_origin_id         = "live-api"
      viewer_protocol_policy   = "https-only"
      allowed_methods          = ["GET", "HEAD"]
      cached_methods           = ["GET", "HEAD"]
      compress                 = false
      cache_policy_id          = data.aws_cloudfront_cache_policy.caching_disabled.id
      origin_request_policy_id = data.aws_cloudfront_origin_request_policy.all_viewer_except_host.id
    }
  }

  default_cache_behavior {
    target_origin_id       = "s3-site"
    viewer_protocol_policy = "redirect-to-https"
    allowed_methods        = ["GET", "HEAD"]
    cached_methods         = ["GET", "HEAD"]
    compress               = true
    cache_policy_id        = data.aws_cloudfront_cache_policy.caching_optimized.id

    function_association {
      event_type   = "viewer-request"
      function_arn = aws_cloudfront_function.spa.arn
    }
  }

  restrictions {
    geo_restriction {
      restriction_type = "none"
    }
  }

  viewer_certificate {
    cloudfront_default_certificate = !var.enable_custom_domain
    acm_certificate_arn            = one(aws_acm_certificate_validation.site[*].certificate_arn)
    ssl_support_method             = var.enable_custom_domain ? "sni-only" : null
    minimum_protocol_version       = var.enable_custom_domain ? "TLSv1.2_2021" : "TLSv1"
  }
}

# ---------------------------------------------------------------- Hostname records (phase 2)

resource "aws_route53_record" "site_alias" {
  for_each = local.site_alias_records

  zone_id = aws_route53_zone.site.zone_id
  name    = each.value.name
  type    = each.value.type

  alias {
    name                   = aws_cloudfront_distribution.site.domain_name
    zone_id                = aws_cloudfront_distribution.site.hosted_zone_id
    evaluate_target_health = false
  }
}
