---
name: elixir-dev
description: "Use when working on Elixir projects for the full development cycle: running tests, debugging with ad-hoc scripts, formatting, and linting with Credo"
---

# Elixir Development Workflow

## Overview

Standard Elixir development cycle: test → debug → format → lint → commit. Designed for projects using Mix with ExUnit, Credo, and optional debug scripts.

## The Cycle

### 1. Run Tests

```powershell
cd <project_dir>; mix test 2>&1
```

**Targeted test runs:**
```powershell
# Single test file
cd <project_dir>; mix test test/sm3_test.exs 2>&1

# Single test by line number
cd <project_dir>; mix test test/sm4_test.exs --only line:23 2>&1

# All tests in a file
cd <project_dir>; mix test test/sm4_test.exs 2>&1
```

### 2. Debug with Ad-Hoc Scripts

When tests fail or you need to inspect runtime behavior, create temporary `debug*.exs` scripts:

```powershell
cd <project_dir>; mix run debug.exs 2>&1
cd <project_dir>; mix run debug2.exs 2>&1
```

**Debug script template:**
```elixir
# debug.exs - temporary debugging script
result = YourModule.your_function(args)
IO.puts("Result: #{inspect(result)}")
```

**After debugging, clean up:**
```powershell
cd <project_dir>; Remove-Item debug.exs, debug2.exs -ErrorAction SilentlyContinue
```

### 3. Format Code

```powershell
# Check formatting without modifying
cd <project_dir>; mix format --check-formatted 2>&1

# Auto-format
cd <project_dir>; mix format 2>&1
```

### 4. Lint with Credo

```powershell
cd <project_dir>; mix credo --strict 2>&1
```

### 5. Full Verification Sequence

```powershell
cd <project_dir>; mix format --check-formatted 2>&1; mix test 2>&1
```

Or with Credo:
```powershell
cd <project_dir>; mix format 2>&1; mix credo --strict 2>&1; mix test 2>&1
```

## Quick Reference

| Action | Command |
|--------|---------|
| Run all tests | `mix test 2>&1` |
| Run specific file | `mix test test/name_test.exs 2>&1` |
| Run at line | `mix test test/name_test.exs --only line:N 2>&1` |
| Check format | `mix format --check-formatted 2>&1` |
| Auto-format | `mix format 2>&1` |
| Lint | `mix credo --strict 2>&1` |
| Run script | `mix run script.exs 2>&1` |
| Run inline | `mix run -e 'IO.puts("hello")' 2>&1` |

## When to Use

- Implementing new Elixir modules or functions
- Fixing bugs in existing Elixir code
- Refactoring Elixir codebases
- Any Elixir project using Mix build system

## Notes

- Always redirect stderr with `2>&1` to capture compiler warnings
- Use `--only line:N` for focused test debugging
- Clean up `debug*.exs` files after use — they should not be committed
- `mix credo --strict` catches more style issues than default credo
