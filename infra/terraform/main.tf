# The static website (S3 + CloudFront + ACM + Route 53 for firstring.life) is defined in site.tf.
#
# The Rust server remains the owner of call policy, state and provider adapters; it is not
# hosted here. The hosted site has no backend: /api is intentionally not routed.
#
# Possible future AWS server functions (not implemented):
#   module "reputation_api"   -> API Gateway + Lambda + DynamoDB, see
#                                frontend/docs/reputation-contract.md
#   module "transcript_export" -> Lambda reading the Rust server's authenticated API
