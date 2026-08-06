import { invoke } from '@tauri-apps/api/core'
import type {
  Agent, InstallResult, BatchResult, SkillInfo, NativeConfig, SessionInfo, PromptInfo,
  MemoryInfo, DiagnosticResult, InstalledAgent, StatusOverview, AuditInfo, BackupManifest,
  SessionDetail, SessionTemplateInfo, PromptVersionInfo, PromptUsageInfo,
} from '../types'

export function useTauriApi() {
  // Agents
  function listAgents(agentType: string | null = null) {
    return invoke<Agent[]>('list_agents', { agentType })
  }

  function searchAgents(query: string, agentType: string | null = null) {
    return invoke<Agent[]>('search_agents', { query, agentType })
  }

  function installAgent(name: string) {
    return invoke<InstallResult>('install_agent', { name })
  }

  function uninstallAgent(name: string) {
    return invoke<InstallResult>('uninstall_agent', { name })
  }

  function batchInstallAgents(names: string[]) {
    return invoke<BatchResult>('batch_install_agents', { names })
  }

  function batchUninstallAgents(names: string[]) {
    return invoke<BatchResult>('batch_uninstall_agents', { names })
  }

  function listInstalledAgents() {
    return invoke<InstalledAgent[]>('list_installed_agents')
  }

  // Config
  function getNativeConfig(agentId: string) {
    return invoke<NativeConfig>('get_native_config', { agentId })
  }

  function saveNativeConfig(agentId: string, content: string) {
    return invoke<void>('save_native_config', { agentId, content })
  }

  // Skills
  function listSkills() {
    return invoke<SkillInfo[]>('list_skills')
  }

  function createSkill(name: string, description: string) {
    return invoke<SkillInfo>('create_skill', { name, description })
  }

  function enableSkill(name: string) {
    return invoke<void>('enable_skill', { name })
  }

  function disableSkill(name: string) {
    return invoke<void>('disable_skill', { name })
  }

  function deleteSkill(name: string) {
    return invoke<boolean>('delete_skill', { name })
  }

  // Sessions
  function listSessions() {
    return invoke<SessionInfo[]>('list_sessions')
  }

  function createSession(title: string, agent: string) {
    return invoke<SessionInfo>('create_session', { title, agent })
  }

  function getSession(id: string) {
    return invoke<SessionInfo>('get_session', { id })
  }

  function deleteSession(id: string) {
    return invoke<boolean>('delete_session', { id })
  }

  // Prompts
  function listPrompts() {
    return invoke<PromptInfo[]>('list_prompts')
  }

  function createPrompt(id: string, name: string, description: string, template: string) {
    return invoke<PromptInfo>('create_prompt', { id, name, description, template })
  }

  function renderPrompt(id: string, vars: Record<string, string>) {
    return invoke<string>('render_prompt', { id, vars })
  }

  function deletePrompt(id: string) {
    return invoke<boolean>('delete_prompt', { id })
  }

  // Memory
  function listMemories(scope: string | null = null) {
    return invoke<MemoryInfo[]>('list_memories', { scope })
  }

  function createMemory(title: string, content: string, scope: string) {
    return invoke<MemoryInfo>('create_memory', { title, content, scope })
  }

  function searchMemories(query: string) {
    return invoke<MemoryInfo[]>('search_memories', { query })
  }

  function deleteMemory(path: string) {
    return invoke<boolean>('delete_memory', { path })
  }

  // Diagnostic
  function runDiagnostics() {
    return invoke<DiagnosticResult>('run_diagnostics')
  }

  // Management: status / audit / backup
  function getStatusOverview() {
    return invoke<StatusOverview>('get_status_overview')
  }

  function listAudit(action: string | null = null, target: string | null = null, sinceDays: number | null = null, limit: number | null = 50) {
    return invoke<AuditInfo[]>('list_audit', { action, target, sinceDays, limit })
  }

  function clearAudit() {
    return invoke<void>('clear_audit')
  }

  function createBackup(outputPath: string) {
    return invoke<BackupManifest>('create_backup', { outputPath })
  }

  function restoreBackup(inputPath: string) {
    return invoke<BackupManifest>('restore_backup', { inputPath })
  }

  // Session: usage / replay / templates
  function replaySession(id: string) {
    return invoke<string>('replay_session', { id })
  }

  function recordSessionUsage(id: string, inputTokens: number, outputTokens: number) {
    return invoke<SessionDetail>('record_session_usage', { id, inputTokens, outputTokens })
  }

  function setSessionModel(id: string, model: string) {
    return invoke<SessionDetail>('set_session_model', { id, model })
  }

  function listSessionTemplates() {
    return invoke<SessionTemplateInfo[]>('list_session_templates')
  }

  function createSessionTemplate(id: string, name: string, description: string, agent: string | null, messages: [string, string][], tags: string[]) {
    return invoke<SessionTemplateInfo>('create_session_template', { id, name, description, agent, messages, tags })
  }

  function createSessionFromTemplate(templateId: string, title: string) {
    return invoke<SessionDetail>('create_session_from_template', { templateId, title })
  }

  function deleteSessionTemplate(id: string) {
    return invoke<boolean>('delete_session_template', { id })
  }

  // Prompt: versions / usage
  function listPromptVersions(id: string) {
    return invoke<PromptVersionInfo[]>('list_prompt_versions', { id })
  }

  function rollbackPrompt(id: string, version: number) {
    return invoke<PromptInfo>('rollback_prompt', { id, version })
  }

  function getPromptUsage() {
    return invoke<PromptUsageInfo[]>('get_prompt_usage')
  }

  function renderPromptChecked(id: string, vars: Record<string, string>) {
    return invoke<string>('render_prompt_checked', { id, vars })
  }

  // Memory: semantic search / decay
  function searchMemoriesSemantic(query: string, topK: number | null = 20) {
    return invoke<MemoryInfo[]>('search_memories_semantic', { query, topK })
  }

  function applyMemoryDecay(olderThanDays: number) {
    return invoke<number>('apply_memory_decay', { olderThanDays })
  }

  function setMemoryImportance(path: string, importance: number) {
    return invoke<void>('set_memory_importance', { path, importance })
  }

  function reviveMemory(path: string) {
    return invoke<void>('revive_memory', { path })
  }

  return {
    listAgents, searchAgents, installAgent, uninstallAgent,
    batchInstallAgents, batchUninstallAgents, listInstalledAgents,
    getNativeConfig, saveNativeConfig,
    listSkills, createSkill, enableSkill, disableSkill, deleteSkill,
    listSessions, createSession, getSession, deleteSession,
    listPrompts, createPrompt, renderPrompt, deletePrompt,
    listMemories, createMemory, searchMemories, deleteMemory,
    runDiagnostics,
    getStatusOverview, listAudit, clearAudit, createBackup, restoreBackup,
    replaySession, recordSessionUsage, setSessionModel,
    listSessionTemplates, createSessionTemplate, createSessionFromTemplate, deleteSessionTemplate,
    listPromptVersions, rollbackPrompt, getPromptUsage, renderPromptChecked,
    searchMemoriesSemantic, applyMemoryDecay, setMemoryImportance, reviveMemory,
  }
}
