#!/usr/bin/env python3
"""Generate the "支持的 Agent" support-matrix section for README.md from agents.json.

Usage:
  python scripts/generate-support-matrix.py          # print section to stdout
  python scripts/generate-support-matrix.py --write  # update README.md in place

Single source of truth: agents.json. Run after catalog changes to keep README in sync.
"""

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
AGENTS_JSON = ROOT / "agents.json"
README_MD = ROOT / "README.md"

# Display-name mapping for package managers
MANAGER_LABELS = {
    "brew-cask": "brew",
    "manual": "—",
}

STATUS_SECTION = """### 支持状态说明

| 状态 | 含义 |
|------|------|
| `verified` | 官方验证过安装流程，信息可靠 |
| `community` | 社区贡献，未经官方验证 |
| `manual` | 无可靠包管理器来源，仅提供官网链接 |
| `deprecated` | 已废弃，不再维护 |
"""


def load_agents():
    with open(AGENTS_JSON, encoding="utf-8") as f:
        data = json.load(f)
    return data["agents"]


def manager_label(manager: str) -> str:
    return MANAGER_LABELS.get(manager, manager)


def first_non_manual(agent):
    """Return the first non-manual installer (stable platform order), else None."""
    for platform in ("windows", "macos", "linux"):
        inst = agent["installers"].get(platform)
        if inst and inst.get("manager") != "manual":
            return inst
    return None


def render_table(rows):
    header = rows[0]
    lines = [
        "| " + " | ".join(header) + " |",
        "|" + "|".join("---" for _ in header) + "|",
    ]
    for row in rows[1:]:
        lines.append("| " + " | ".join(row) + " |")
    return "\n".join(lines)


def generate(agents):
    cli = [a for a in agents if a["kind"] == "cli"]
    desktop = [a for a in agents if a["kind"] == "desktop"]

    cli_rows = [["Agent", "提供商", "包名", "包管理器", "状态"]]
    for a in sorted(cli, key=lambda x: x["name"].lower()):
        inst = first_non_manual(a)
        pkg = (inst or {}).get("package") or "—"
        mgr = manager_label((inst or {}).get("manager") or "manual")
        cli_rows.append([a["name"], a["provider"], f"`{pkg}`", mgr, a["status"]])

    desktop_rows = [["Agent", "提供商", "Windows", "macOS", "状态"]]
    for a in sorted(desktop, key=lambda x: x["name"].lower()):
        win = manager_label(a["installers"].get("windows", {}).get("manager", "—"))
        mac = manager_label(a["installers"].get("macos", {}).get("manager", "—"))
        desktop_rows.append([a["name"], a["provider"], win, mac, a["status"]])

    lines = [
        "## 支持的 Agent",
        "",
        "> 完整目录见 [agents.json](agents.json)，Schema 定义见 [agents.schema.json](agents.schema.json)",
        "",
        f"### CLI Agent（{len(cli_rows) - 1} 个）",
        "",
        render_table(cli_rows),
        "",
        f"### Desktop Agent（{len(desktop_rows) - 1} 个）",
        "",
        render_table(desktop_rows),
        "",
        STATUS_SECTION.strip(),
    ]
    return "\n".join(lines) + "\n"


def write_to_readme(section: str):
    text = README_MD.read_text(encoding="utf-8")
    pattern = re.compile(r"## 支持的 Agent.*?(?=^## )", re.MULTILINE | re.DOTALL)
    if not pattern.search(text):
        sys.exit("ERROR: could not find '## 支持的 Agent' section in README.md")
    updated = pattern.sub(section, text)
    README_MD.write_text(updated, encoding="utf-8", newline="\n")
    print(f"[OK] README.md support matrix updated ({README_MD})")


def main():
    agents = load_agents()
    section = generate(agents)

    cli_count = sum(1 for a in agents if a["kind"] == "cli")
    desktop_count = sum(1 for a in agents if a["kind"] == "desktop")
    print(f"# agents.json: {len(agents)} agents ({cli_count} CLI, {desktop_count} Desktop)\n")
    print(section)

    if "--write" in sys.argv:
        write_to_readme(section)


if __name__ == "__main__":
    main()
