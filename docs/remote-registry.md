# Remote registries

AgentHub supports an intentionally small, offline-first JSON registry protocol
for community prompts and skills. The local directories remain the source of
truth when a remote is unavailable.

## Prompt registry

`GET` and `POST` endpoints exchange either a prompt array or this envelope:

```json
{
  "version": 1,
  "prompts": [
    {
      "id": "review",
      "name": "Review",
      "description": "Review code",
      "template": "Review {{code}}",
      "variables": [],
      "tags": ["quality"],
      "category": "development",
      "version": 1,
      "publisher": "alice",
      "published_at": "2026-08-14T00:00:00Z",
      "source": "review"
    }
  ]
}
```

Pull only adds new snapshots and updates a snapshot when the remote version is
newer. `--force` overrides that rule. Push sends the complete local snapshot
set; a server should authenticate the request and apply its own conflict and
publication policy.

```bash
agenthub prompt community pull https://registry.example/prompts \
  --token "$AGENTHUB_REMOTE_TOKEN"
agenthub prompt community push https://registry.example/prompts \
  --token "$AGENTHUB_REMOTE_TOKEN"
```

## Skill registry

The skill registry uses the same GET/POST pattern with a `packages` envelope.
Each package contains UTF-8 files under relative paths and must include a
`SKILL.md` whose manifest name matches the package name:

```json
{
  "version": 1,
  "packages": [
    {
      "name": "rust-dev",
      "version": "1.0.0",
      "description": "Rust workflow",
      "author": "alice",
      "tags": ["rust"],
      "files": {
        "SKILL.md": "---\nname: rust-dev\nversion: 1.0.0\n---\n..."
      }
    }
  ]
}
```

For safety, the first protocol version accepts only UTF-8 files, rejects
absolute paths, `.`/`..` traversal, hidden paths, packages larger than 1024
files, and files larger than 4 MiB. It does **not** distribute plugins.
Plugins execute commands and require a separate signed/trusted package format.

```bash
agenthub skill market pull https://registry.example/skills \
  --token "$AGENTHUB_REMOTE_TOKEN"
agenthub skill market push https://registry.example/skills \
  --token "$AGENTHUB_REMOTE_TOKEN"
```

## Plugin registry

Plugins can execute commands, so their registry installs are strictly
opt-in. The envelope uses a `plugins` array:

```json
{
  "version": 1,
  "plugins": [
    {
      "name": "release-notifier",
      "version": "1.0.0",
      "description": "Notify on releases",
      "author": "alice",
      "files": {
        "plugin.yaml": "name: release-notifier\nversion: 1.0.0\nhooks:\n- event: on_monitor\n  command: \"sh notify.sh\"\n  args: []\n",
        "notify.sh": "#!/bin/sh\necho notifying"
      }
    }
  ]
}
```

Pulled packages are installed **disabled**: the `.enabled` marker is written
only by `plugin enable`, and `run_hook` never executes hooks of disabled
plugins. The `.enabled` marker is also never exported on push — enablement is
per-install. Validation rejects unknown hook events, entry paths escaping the
package directory, traversal/backslash/hidden paths, packages over 1024
files, and files over 4 MiB or non-UTF-8.

```bash
agenthub plugin pull https://registry.example/plugins \
  --token "$AGENTHUB_REMOTE_TOKEN"
agenthub plugin push https://registry.example/plugins \
  --token "$AGENTHUB_REMOTE_TOKEN"
```

Signature verification and revocation lists for plugins are still future
work; until then, only pull from registries you trust and enable plugins
deliberately.

Both transports accept `http://` for local development and `https://` for
real deployments. Requests have bounded connect/total timeouts and a bearer
token can be supplied with `--token` or `AGENTHUB_REMOTE_TOKEN`.
