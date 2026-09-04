# AGENTS.md — mcp

## Serves

- **O1 — governed reach.** A consumer can discover and call an MCP tool without trusting the
  server's annotations as authority, and every bound refuses by name.
- **O5 — the generic agent platform.** Harness and Connectors share one MCP client implementation
  while retaining their own policy and credential-custody boundaries.

## Boundaries

- This repository owns MCP client protocol compatibility, standard transports, OAuth mechanics,
  local named connections, and reusable conformance fixtures.
- It owns no agent loop, connector catalog, grant, approval policy, durable hosted secret store, or
  model-provider wire.
- `b10x-mcp-types` performs no I/O, reads no clock, and carries no credential value.
- Server annotations are untrusted hints. A consumer supplies all authority and effect policy.
- Unknown result content is preserved. No frame, schema, argument, result, or OAuth response is
  silently truncated.
- Anything that runs is Rust. No crate or module is named `common`, `shared`, `utils`, `misc`, or
  `helpers`.

## Credentials

- A credential value has no `Display`, has redacted `Debug`, and is zeroized on drop.
- Configurations contain credential sources, never credential values.
- OAuth client credentials are bound to the authorization-server issuer and MCP resource.
- Stdio receives only explicitly named environment variables and never runs through a shell.

## Contracts and gate

Released contract directories are immutable. A wire-visible change cuts a new directory and enters
`CHANGELOG.md`. Run `cargo xtask gate` before every commit.

<!-- b10x-docs-operations:start -->
## Public documentation operations

This repository owns the public source and presentation allowlist in `b10x.docs.yaml`. The generated credential-free `.github/workflows/b10x-docs-bundle.yml` passively packages only those declared files for the exact successful `main` commit; it must never run repository code. Atlas selects the latest successful bundle with every other catalog source, and Website plus Docs System own rendering, shared components, search, and feeds. Do not add a standalone docs deployer or put App credentials in this public repository. If Atlas catalogs a former Pages workflow, that file remains repository-owned validation: preserve its bespoke checks while keeping exact read-only permissions, an unconditional pull-request trigger, and no deployment primitives. Project Pages at `/mcp/` is only the generated stable redirect façade in `.github/workflows/b10x-docs-pages.yml`; content-only publication never rebuilds it.

From the complete organization workspace, verify the contract with a clean Atlas checkout at the current remote `main`. Set `B10X_ATLAS_CHECKOUT` to a managed Atlas worktree when the primary checkout is dirty or stale; never infer command availability from the primary alone.

```bash
atlas_checkout="${B10X_ATLAS_CHECKOUT:-atlas}"
atlas_head="$(git -C "$atlas_checkout" rev-parse HEAD)"
atlas_main="$(git -C "$atlas_checkout" ls-remote origin refs/heads/main | awk '{print $1}')"
test -z "$(git -C "$atlas_checkout" status --porcelain)"
test "$atlas_head" = "$atlas_main"
cargo run --manifest-path "$atlas_checkout/Cargo.toml" --locked -q -- \
  --store "$atlas_checkout/catalog/store" docs reconcile --workspace . --check
```

Keep internal plans, stories, ADRs, decisions, worklogs, security material, and research out of the public allowlist unless a repository authority explicitly declares them public.
<!-- b10x-docs-operations:end -->
