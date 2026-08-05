// === Agent Types ===
export interface InstallerInfo {
  platform: string
  manager: string
  package: string | null
}

export interface Agent {
  id: string
  name: string
  description: string
  kind: 'CLI' | 'Desktop'
  provider: string
  homepage: string
  status: string
  installers: InstallerInfo[]
  catalog_verified_at: string | null
  installer_verified_at: string | null
}

export interface InstallResult {
  success: boolean
  message: string
  agent_name: string
  command: string
  exit_code: number | null
  stdout: string
  stderr: string
  duration_ms: number
  timed_out: boolean
}

export interface BatchResult {
  total: number
  success: number
  failed: number
  results: InstallResult[]
}

export interface InstalledAgent {
  id: string
  installed: boolean
  version: string | null
}

// === Skill Types ===
export interface SkillInfo {
  name: string
  description: string
  version: string
  enabled: boolean
  tags: string[]
  category: string | null
  source: string
}

// === Config Types ===
export interface NativeConfig {
  agent_id: string
  config_path: string
  config_content: string
  config_format: string
  parsed: object | null
}

// === Session Types ===
export interface SessionInfo {
  id: string
  title: string
  agent: string
  status: string
  started_at: string
  ended_at: string | null
  message_count: number
  tags: string[]
}

// === Prompt Types ===
export interface PromptInfo {
  id: string
  name: string
  description: string
  template: string
  tags: string[]
  category: string | null
  version: number
}

// === Memory Types ===
export interface MemoryInfo {
  path: string
  title: string
  content: string
  scope: string
  memory_type: string
  tags: string[]
  updated_at: string
}

// === Diagnostic Types ===
export interface CheckResult {
  name: string
  category: string
  status: string
  message: string
}

export interface DiagnosticResult {
  summary: string
  checks: CheckResult[]
  passed: number
  warnings: number
  failed: number
}

// === Progress Types ===
export interface CardProgress {
  step: number
  total_steps: number
  message: string
}

export interface BatchProgress {
  current: number
  total: number
  agent: string
  action: string
}
