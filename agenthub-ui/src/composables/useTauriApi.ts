import { invoke } from '@tauri-apps/api/core'
import type { Agent, InstallResult, BatchResult, SkillInfo, NativeConfig, SessionInfo, PromptInfo, MemoryInfo, DiagnosticResult, InstalledAgent } from '../types'

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

  return {
    listAgents, searchAgents, installAgent, uninstallAgent,
    batchInstallAgents, batchUninstallAgents, listInstalledAgents,
    getNativeConfig, saveNativeConfig,
    listSkills, createSkill, enableSkill, disableSkill, deleteSkill,
    listSessions, createSession, getSession, deleteSession,
    listPrompts, createPrompt, renderPrompt, deletePrompt,
    listMemories, createMemory, searchMemories, deleteMemory,
    runDiagnostics,
  }
}
