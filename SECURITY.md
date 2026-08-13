# Security Policy

## Supported Versions

| Version | Supported          |
|---------|--------------------|
| 1.x     | :white_check_mark: |
| < 1.0   | :x:                |

## Reporting a Vulnerability

If you discover a security vulnerability in AgentHub, report it privately
through the repository's **Security → Report a vulnerability** form:
<https://github.com/ZeroMarker/agenthub/security/advisories/new>.

**Do not** report security vulnerabilities through public GitHub issues.

Please include:

- A description of the vulnerability
- Steps to reproduce
- Affected versions
- Any potential impact

You should receive a response within 48 hours. If the issue is confirmed, we will release a patch as soon as possible, typically within the next release cycle.

## Scope

AgentHub executes system commands (npm, pip, winget, brew) to install and uninstall agents. Key security considerations:

- **Command injection**: All external commands use structured arguments, not shell concatenation. User-provided agent names are validated before passing to commands.
- **Filesystem boundaries**: Identifiers used as persisted path components reject traversal, path separators, control characters, and unsafe relative paths. Backup and import flows reuse the same validation.
- **Supply chain**: Install commands use official package registries. Manual installers only open the agent's homepage — no auto-execution of untrusted scripts.
- **Permissions**: AgentHub does not request elevated privileges. All operations run at the user's permission level.

## Code Signing

AgentHub release binaries and installers are currently **unsigned**. Every
release includes SHA-256 checksums so users can verify download integrity. The
planned platform signing approach is documented in `docs/signing-policy.md`.

### Windows

- Windows binaries and installers are currently unsigned and may trigger
  SmartScreen warnings. Verify their published SHA-256 checksums before use.

### macOS

- macOS binaries and application bundles are currently unsigned and not
  notarized. Gatekeeper may require an explicit user override after checksum
  verification.

### Linux

- Linux AppImage, DEB and RPM artifacts are currently unsigned. Verify the
  published SHA-256 checksums before installation.

### Checksums

Every release publishes platform-specific SHA-256 checksum files. Download the
checksum file attached alongside the artifact and verify before use:

```bash
sha256sum -c SHA256SUMS-<target>
```

### CI/CD Integrity

All release builds run on GitHub Actions hosted runners. Build steps are
defined in `.github/workflows/release.yml` and are reproducible from the tagged
commit. Release notes must disclose the current unsigned status; checksums are
generated and attached by the release workflow.

## Security Best Practices

1. Always review `--dry-run` output before running install/uninstall commands
2. Only install agents from trusted sources (verified catalog entries)
3. Keep AgentHub updated to the latest version
4. Report suspicious package behavior through the issue tracker
5. Verify SHA-256 checksums before running downloaded release artifacts
6. Treat operating-system warnings for unsigned artifacts seriously and only
   override them after confirming the checksum and release source
