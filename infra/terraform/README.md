# Infrastructure (Terraform, AWS) — scaffold only

Prepared in case FirstRing needs AWS server functions. **Nothing is deployed and no resources are defined.**
The Rust server remains the owner of call policy, state and provider adapters (see `AGENTS.md`).

Installed locally: Terraform v1.17.1 (Homebrew, `hashicorp/tap/terraform`).

```sh
cd infra/terraform
terraform init          # downloads the pinned AWS provider; no AWS credentials needed
terraform validate
terraform plan          # needs AWS credentials; currently plans zero resources
```

Rules:
- Credentials come from the standard AWS chain (`AWS_PROFILE`/SSO). Never commit keys, `*.tfvars` or state.
- State is local until a shared S3 backend is chosen; `.gitignore` excludes it.
- Any new resource needs explicit authorization for account spend before `apply`.
- `.terraform.lock.hcl` is committed so provider versions stay pinned.
