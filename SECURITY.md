# Security Policy

## Supported Versions

| Version | Supported          |
|---------|--------------------|
| 1.x     | :white_check_mark: |
| < 1.0   | :x:                |

## Reporting a Vulnerability

If you discover a security vulnerability in AgentHub, please report it privately by emailing **security@agenthub.dev** (placeholder — replace with actual address before release).

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
- **Supply chain**: Install commands use official package registries. Manual installers only open the agent's homepage — no auto-execution of untrusted scripts.
- **Permissions**: AgentHub does not request elevated privileges. All operations run at the user's permission level.

## Code Signing

AgentHub release binaries and installers are signed to ensure authenticity and integrity.

### Windows

- **CLI binaries**: Signed with an Extended Validation (EV) code signing certificate via Azure Key Vault during the release workflow.
- **Desktop installer (MSI)**: The Tauri-bundled MSI is signed with the same certificate. Windows SmartScreen will show the publisher name on first run.
- **Signature verification**: Run `Get-AuthenticodeSignature agenthub-cli.exe` in PowerShell to verify.

### macOS

- **CLI binaries**: Signed and notarized by Apple Notary Service using an Apple Developer ID certificate.
- **Desktop app (DMG)**: The `.dmg` is signed and notarized. Gatekeeper will allow installation without override.
- **Signature verification**: Run `codesign -dv --verbose=4 /path/to/agenthub` and `spctl -a -t exec -vv /path/to/agenthub`.

### Linux

- **AppImage**: GPG-signed using the project's release key. Verify with `gpg --verify agenthub-x86_64.AppImage.asc`.
- **DEB/RPM packages**: Signed with a Debian-compliant signing key at build time.

### Checksums

Every release publishes SHA-256 checksums for all artifacts in a `SHA256SUMS` file. Verify before use:

```bash
# Compare your download against the published checksum
sha256sum agenthub-cli-x86_64-pc-windows-msvc.zip
# Expected output should match the SHA256SUMS file
```

### CI/CD Integrity

All release builds run on GitHub Actions hosted runners. Build steps are defined in `.github/workflows/release.yml` and are reproducible via the tagged commit. No unsigned artifacts are published to release pages.

## Security Best Practices

1. Always review `--dry-run` output before running install/uninstall commands
2. Only install agents from trusted sources (verified catalog entries)
3. Keep AgentHub updated to the latest version
4. Report suspicious package behavior through the issue tracker
5. Verify SHA-256 checksums before running downloaded release artifacts
6. On macOS, verify code signatures with `codesign` and `spctl` before first launch
