# rust-skgen

`rust-skgen` creates and refreshes AI-agent skills from public documentation.
It retrieves HTML documentation, follows links within a configured scope, and writes a
deterministic skill with attributed source URLs and metadata for later updates.

## Requirements

- Rust 1.85 or later.
- Network access to the public documentation site.

The CLI respects `robots.txt`, does not follow redirects, and rejects URLs that require
credentials. Only use it with documentation that you are permitted to retrieve.

## Build and run

Run commands from the directory where you want the `.agents/skills` directory created.

```powershell
cargo run -- --help
```

Build a reusable release binary:

```powershell
cargo build --release
& (Join-Path $PWD 'target\release\rust-skgen.exe') --help
```

## Create a skill

```powershell
cargo run -- create https://www.radix-ui.com/primitives/docs/overview/introduction radix-primitives
```

On success, the generated files are placed below the current working directory:

```text
.agents/skills/radix-primitives/
  SKILL.md
  metadata.json
```

The skill name must be a slug of up to 64 lowercase ASCII letters, digits, and hyphens.
It cannot start or end with a hyphen. Existing skill directories are never overwritten by
`create`.

## Discovery options

`create` accepts these options. The same options can be changed for one selected skill with
`update`.

```text
--scope same-site|path-prefix|parent-directory|documentation-navigation
--site-boundary exact-host|base-domain|same-origin
--allow-subdomain <host>
--traversal all|one-level|limited
--max-pages <positive-integer>
--format guide-with-references|organized-content
--user-agent <value>
```

Defaults:

```text
scope:          same-site
site boundary:  exact-host
traversal:      all
format:         guide-with-references
max pages:      100, when --traversal limited omits --max-pages
```

Scope behavior:

- `same-site` follows pages within the selected site boundary.
- `path-prefix` follows pages under the source URL path prefix.
- `parent-directory` follows pages under the source URL parent directory.
- `documentation-navigation` follows only links in navigation elements on the source page.

Site boundaries are valid only with `same-site`:

- `exact-host` keeps the same host and port, while allowing HTTP/HTTPS changes.
- `base-domain` allows registered-domain pages. A different subdomain must be listed with
  `--allow-subdomain` and linked from an included page.
- `same-origin` requires the same scheme, host, and port.

Examples:

```powershell
# Extract only direct links from the source page.
cargo run -- create https://example.com/docs/start example-docs --traversal one-level

# Extract up to 25 pages under the source path prefix as organized content.
cargo run -- create https://example.com/docs/api/start api-docs `
  --scope path-prefix `
  --traversal limited `
  --max-pages 25 `
  --format organized-content

# Permit an explicitly linked subdomain while using the base-domain boundary.
cargo run -- create https://docs.example.com/start example-docs `
  --scope same-site `
  --site-boundary base-domain `
  --allow-subdomain api.example.com
```

## Update skills

Update every managed skill in the current directory:

```powershell
cargo run -- update
```

Update selected skills without changing their stored configuration:

```powershell
cargo run -- update radix-primitives another-skill
```

Change configuration for exactly one selected skill:

```powershell
cargo run -- update radix-primitives `
  --name radix-docs `
  --source-url https://www.radix-ui.com/primitives/docs/overview/introduction `
  --scope same-site `
  --site-boundary exact-host `
  --traversal limited `
  --max-pages 25 `
  --format organized-content
```

Configuration changes are rejected if no skill or more than one skill is selected.

If managed metadata is missing, invalid, or no longer matches `SKILL.md`, the CLI asks:

```text
Rebuild metadata and update this skill? [y/N]
```

Enter `y` to rebuild and update that skill. Any other response, including EOF, preserves it
unchanged. During `update` without names, the prompt is shown once for each affected skill.

## Output and exit codes

Successful operations write to standard output:

```text
Created skill: <name>
Updated skill: <name>
No managed skills found.
```

Skill failures write their cause to standard error:

```text
Failed skill: <name>: <reason>
```

| Exit code | Meaning |
| --- | --- |
| `0` | All requested operations succeeded, including no managed skills found. |
| `1` | One or more skills failed. Batch updates continue with remaining skills. |
| `2` | Invalid arguments or an invalid option combination. |

## Help

```powershell
cargo run -- create --help
cargo run -- update --help
```

## Development checks

```powershell
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```
