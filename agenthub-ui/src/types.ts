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

// === Management / Dashboard Types ===
export interface CatalogOverview {
  total: number
  cli: number
  desktop: number
  verified: number
  community: number
  manual: number
  deprecated: number
}

export interface SessionStats {
  total: number
  active: number
  completed: number
  failed: number
  total_tokens: number
  total_cost: number
}

export interface MemoryStats {
  total: number
  global: number
  project: number
  session: number
  decayed: number
}

export interface StatusOverview {
  generated_at: string
  platform: string
  agenthub_version: string
  catalog: CatalogOverview
  installed_agents: number
  configs: number
  prompts: number
  sessions: SessionStats
  memories: MemoryStats
  skills_total: number
  skills_enabled: number
  audit_events: number
}

export interface AuditInfo {
  id: string
  timestamp: string
  actor: string
  action: string
  target: string
  details: string | null
  success: boolean
}

export interface BackupCounts {
  configs: number
  prompts: number
  prompt_versions: number
  sessions: number
  session_templates: number
  memories: number
  audit_events: number
}

export interface BackupManifest {
  format_version: number
  created_at: string
  agenthub_version: string
  counts: BackupCounts
}

// === Session Detail (usage / replay) ===
export interface SessionDetail extends SessionInfo {
  model: string | null
  total_tokens: number
  estimated_cost_usd: number
}

export interface SessionTemplateInfo {
  id: string
  name: string
  description: string
  agent: string | null
  message_count: number
  tags: string[]
}

// === Prompt versions / usage ===
export interface PromptVersionInfo {
  version: number
  name: string
  description: string
  template: string
  updated_at: string | null
}

export interface PromptUsageInfo {
  id: string
  name: string
  usage_count: number
  last_used_at: string | null
}
