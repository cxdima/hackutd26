#!/usr/bin/env bash
# Build the Angular console and publish it to the FirstRing S3 bucket behind CloudFront.
#
# Usage: infra/deploy-site.sh [--skip-build] [--no-wait]
#   --skip-build  upload the existing frontend/dist output without rebuilding
#   --no-wait     do not wait for the CloudFront invalidation to finish
#
# Bucket and distribution come from `terraform output` in infra/terraform, or from
# SITE_BUCKET / SITE_DISTRIBUTION_ID. AWS credentials come from the standard AWS chain.
#
# Cache policy:
#   *.html (index.html)                      no-cache, max-age=0
#   hashed root *.js / *.css and media/*     public, max-age=31536000, immutable
#   anything else (favicon.svg, ...)         public, max-age=3600
# media/ files that are not content-hashed must be renamed when their content changes, otherwise
# browsers that already cached them keep the old copy for up to a year.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TF_DIR="$REPO_ROOT/infra/terraform"
FRONTEND_DIR="$REPO_ROOT/frontend"
DIST="$FRONTEND_DIR/dist/firstring-web/browser"

build=1
wait_for_invalidation=1
for arg in "$@"; do
  case "$arg" in
    --skip-build) build=0 ;;
    --no-wait) wait_for_invalidation=0 ;;
    -h | --help)
      sed -n '2,17p' "$0"
      exit 0
      ;;
    *)
      echo "unknown argument: $arg" >&2
      exit 2
      ;;
  esac
done

BUCKET="${SITE_BUCKET:-$(terraform -chdir="$TF_DIR" output -raw bucket_name)}"
DISTRIBUTION_ID="${SITE_DISTRIBUTION_ID:-$(terraform -chdir="$TF_DIR" output -raw distribution_id)}"
SITE_URL="$(terraform -chdir="$TF_DIR" output -raw site_url 2>/dev/null || true)"

if [[ "$build" == 1 ]]; then
  echo "==> Building frontend"
  npm --prefix "$FRONTEND_DIR" run build
fi

if [[ ! -f "$DIST/index.html" ]]; then
  echo "missing $DIST/index.html; build the frontend first" >&2
  exit 1
fi

IMMUTABLE="public, max-age=31536000, immutable"
SHORT="public, max-age=3600"
NO_CACHE="no-cache, max-age=0"

content_type() {
  case "$1" in
    html) echo "text/html; charset=utf-8" ;;
    js | mjs) echo "text/javascript; charset=utf-8" ;;
    css) echo "text/css; charset=utf-8" ;;
    json | map) echo "application/json; charset=utf-8" ;;
    webmanifest) echo "application/manifest+json; charset=utf-8" ;;
    txt) echo "text/plain; charset=utf-8" ;;
    svg) echo "image/svg+xml" ;;
    png) echo "image/png" ;;
    jpg | jpeg) echo "image/jpeg" ;;
    gif) echo "image/gif" ;;
    webp) echo "image/webp" ;;
    avif) echo "image/avif" ;;
    ico) echo "image/x-icon" ;;
    woff2) echo "font/woff2" ;;
    woff) echo "font/woff" ;;
    ttf) echo "font/ttf" ;;
    mp4) echo "video/mp4" ;;
    webm) echo "video/webm" ;;
    mp3) echo "audio/mpeg" ;;
    wav) echo "audio/wav" ;;
    ogg) echo "audio/ogg" ;;
    wasm) echo "application/wasm" ;;
    pdf) echo "application/pdf" ;;
    *) echo "" ;;
  esac
}

# upload <cache-control> <content-type or empty> <filter args...>
upload() {
  local cache_control="$1" type="$2"
  shift 2
  local args=(--exclude "*" "$@" --cache-control "$cache_control" --only-show-errors)
  if [[ -n "$type" ]]; then
    args+=(--content-type "$type")
  fi
  aws s3 sync "$DIST" "s3://$BUCKET" "${args[@]}"
}

echo "==> Uploading assets to s3://$BUCKET"
extensions="$(cd "$DIST" && find . -type f -name '*.*' | sed 's/.*\.//' | sort -u)"
for ext in $extensions; do
  [[ "$ext" == "html" ]] && continue
  type="$(content_type "$ext")"
  if [[ -z "$type" ]]; then
    echo "warning: no explicit Content-Type for .$ext; using the AWS CLI guess" >&2
  fi
  # Everything under media/ is long-lived.
  upload "$IMMUTABLE" "$type" --include "media/*.$ext"
  if [[ "$ext" == "js" || "$ext" == "css" ]]; then
    # Angular emits content-hashed chunk/main/styles files at the root.
    upload "$IMMUTABLE" "$type" --include "*.$ext" --exclude "media/*"
  else
    upload "$SHORT" "$type" --include "*.$ext" --exclude "media/*"
  fi
done

echo "==> Uploading HTML (no-cache)"
upload "$NO_CACHE" "$(content_type html)" --include "*.html"

echo "==> Removing stale objects"
# Every current file is already uploaded, so this pass only deletes objects that are gone locally.
aws s3 sync "$DIST" "s3://$BUCKET" --delete --only-show-errors

echo "==> Invalidating CloudFront $DISTRIBUTION_ID"
invalidation_id="$(aws cloudfront create-invalidation \
  --distribution-id "$DISTRIBUTION_ID" \
  --paths "/index.html" "/*" \
  --query 'Invalidation.Id' --output text)"
echo "    invalidation $invalidation_id"
if [[ "$wait_for_invalidation" == 1 ]]; then
  aws cloudfront wait invalidation-completed --distribution-id "$DISTRIBUTION_ID" --id "$invalidation_id"
  echo "    invalidation completed"
fi

echo "==> Deployed ${SITE_URL:-to s3://$BUCKET}"
