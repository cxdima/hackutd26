# Infrastructure (Terraform, AWS): firstring.life static site

Hosts the Angular console (`frontend/`) as a static site: private S3 bucket → CloudFront
(Origin Access Control) → ACM certificate → Route 53 zone for `firstring.life`. Everything is in
us-east-1 in account 851725466918 and tagged `Project = FirstRing`.

The Rust server is **not** hosted here and `/api` is not routed. On the hosted site the console
shows "The FirstRing server isn't running on this computer" with an Explore-the-demo button; that is
expected.

| File | Purpose |
|---|---|
| `site.tf` | Route 53 zone (+ copied `_dmarc` TXT), S3 bucket + policy, ACM certificate + validation records, CloudFront distribution, apex/www alias records |
| `variables.tf` | `domain_name`, `enable_custom_domain` (phase switch), `dmarc_record`, region/environment |
| `outputs.tf` | `name_servers`, `cloudfront_domain_name`, `bucket_name`, `distribution_id`, `site_url`, ... |
| `../deploy-site.sh` | Build the frontend, upload with correct `Content-Type`/`Cache-Control`, invalidate CloudFront |

## Two-phase rollout

ACM cannot validate the certificate until the domain uses the Route 53 name servers, so the domain is
attached in a second apply.

**Phase 1 (applied):** `terraform apply` with `enable_custom_domain = false` (the default) creates the
zone, bucket, certificate request + its DNS validation records, and a CloudFront distribution on the
default `*.cloudfront.net` certificate. The site is live at `terraform output -raw site_url`.

**Registrar step (GoDaddy, manual):** set the domain's name servers to the four values from
`terraform output name_servers`. For the current zone they are:

```
ns-1356.awsdns-41.org
ns-639.awsdns-15.net
ns-1743.awsdns-25.co.uk
ns-303.awsdns-37.com
```

GoDaddy → My Products → firstring.life → DNS → Nameservers → Change nameservers →
"I'll use my own nameservers" → enter the four values (no trailing dots) → Save → confirm.
Propagation usually takes minutes, occasionally a few hours.

Check: `dig NS firstring.life +short` lists the `awsdns` servers, and
`aws acm describe-certificate --region us-east-1 --certificate-arn "$(terraform output -raw certificate_arn)" --query Certificate.Status --output text`
prints `ISSUED` (ACM re-checks every few minutes on its own).

**Phase 2:** attach the hostnames.

```sh
cd infra/terraform
terraform apply -var enable_custom_domain=true
```

To start phase 2 automatically once delegation is visible:

```sh
cd infra/terraform
until dig NS firstring.life +short | grep -q awsdns; do sleep 30; done
terraform apply -var enable_custom_domain=true
```

Phase 2 waits (up to 30 minutes) for the certificate, adds `firstring.life` and `www.firstring.life` to
CloudFront, and creates A/AAAA alias records. CloudFront then redeploys for a few minutes. Keep passing
`-var enable_custom_domain=true` on every later apply (or put `enable_custom_domain = true` in a local,
gitignored `terraform.tfvars`), otherwise Terraform removes the hostnames again.

ACM abandons a pending validation after 72 hours. If the name servers are changed later than that,
run `terraform apply -replace=aws_acm_certificate.site` first.

## Deploying site content

```sh
infra/deploy-site.sh              # npm run build, upload, invalidate, wait
infra/deploy-site.sh --skip-build # upload the existing dist output
infra/deploy-site.sh --no-wait    # do not wait for the invalidation
```

`index.html` is uploaded with `no-cache, max-age=0`; hashed root JS/CSS and everything under `media/`
with `public, max-age=31536000, immutable`; other files (for example `favicon.svg`) with
`public, max-age=3600`. Stale objects are deleted (`--delete`). The invalidation covers `/index.html`
and `/*`. Unhashed files in `media/` must be renamed when their content changes.

SPA deep links (`/console`, `/console/present`, `/console/calls/:id`, ...) are not S3 objects; S3
answers 403, and CloudFront's custom error responses for 403/404 return `/index.html` with status 200.
A missing asset therefore also returns the HTML shell with 200.

## Cost

Route 53 hosted zone $0.50/month plus negligible query, S3 and CloudFront (PriceClass_100) charges at
demo traffic. ACM public certificates are free. No WAF, logging bucket or Lambda@Edge.

To remove everything: `terraform destroy` (pass the same `enable_custom_domain` value). The bucket has
`force_destroy = true`, so its build output is deleted with it. Point the registrar back at GoDaddy DNS
first if the domain should keep resolving.

## Rules

- Credentials come from the standard AWS chain (`AWS_PROFILE`/SSO). Never commit keys, `*.tfvars`,
  plans or state.
- State is local (`terraform.tfstate` in this directory) and gitignored. It is the only record of
  these resources; do not delete it. Move it to an S3 backend before anyone else applies.
- `.terraform.lock.hcl` is committed so the provider version (hashicorp/aws 6.72.0) stays pinned.
- Terraform v1.17.1 (Homebrew, `hashicorp/tap/terraform`).
