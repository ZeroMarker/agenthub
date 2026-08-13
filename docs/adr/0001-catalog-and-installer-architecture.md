# ADR 0001: Catalog and Installer Architecture

- Status: Accepted
- Date: 2026-08-13

## Context

AgentHub must expose the same agent metadata and installation behavior through
the CLI and desktop application while supporting platform-specific package
managers. Duplicated catalogs or ad-hoc command construction would allow the
two clients to drift and could execute an installer that has not been verified
for the current platform.

## Decision

1. `agents.json` is the single source of truth for the supported-agent catalog.
   `agents.schema.json` defines its machine-validated format; generated support
   tables are documentation outputs and are not authoritative inputs.
2. Installation is selected from the catalog's platform-specific installer
   configuration. Package-manager command construction belongs in
   `agenthub-core`; the CLI and Tauri backend call the same core interfaces.
3. Installer support is explicit. Entries without a reliable installer use the
   manual flow and must not synthesize or guess package commands.
4. Every mutating installer operation supports preview and confirmation. Batch
   operations retain an independent result for every target.
5. Catalog and installer identifiers used as storage path components must pass
   the shared safe-identifier validation before filesystem access.

## Consequences

- Catalog changes must update and validate `agents.json` against its schema.
- New package managers require a core adapter plus command-generation and
  output-parsing tests before they can be marked verified.
- Platform differences remain data-driven where possible; client-specific
  copies of catalog or installer logic are not allowed.
- Generated documentation should be refreshed with
  `scripts/generate-support-matrix.py` whenever catalog metadata changes.
