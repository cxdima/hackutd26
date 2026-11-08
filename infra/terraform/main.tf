# Intentionally empty. This scaffold exists so AWS server functions (for example,
# a transcript export Lambda or a reputation API) can be added quickly if the team
# decides to use AWS. Nothing here creates resources; `terraform plan` shows no changes.
#
# Planned shape when needed (not implemented):
#   module "reputation_api"   -> API Gateway + Lambda + DynamoDB, see
#                                frontend/docs/reputation-contract.md
#   module "transcript_export" -> Lambda reading the Rust server's authenticated API
