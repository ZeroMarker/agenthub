<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'

interface AgentInfo {
  id: string
  name: string
  kind: string
  provider: string
}

interface InstalledAgent {
  id: string
  name: string
  installed: boolean
  version: string | null
}

interface NativeConfig {
  agent_id: string
  config_path: string
  config_content: string
  config_format: string
  parsed: Record<string, any> | null
}

const agents = ref<AgentInfo[]>([])
const installedAgents = ref<InstalledAgent[]>([])
const installedLoaded = ref(false)
const selectedAgent = ref<AgentInfo | null>(null)
const nativeConfig = ref<NativeConfig | null>(null)
const loading = ref(false)
const message = ref('')
const messageType = ref<'success' | 'error'>('success')
const searchQuery = ref('')
const showInstalledOnly = ref(false)
const editing = ref(false)
const editContent = ref('')
const editKey = ref('')
const editValue = ref('')

async function loadAgents() {
  loading.value = true
  try {
    agents.value = await invoke<AgentInfo[]>('list_agents', { agentType: null })

    invoke<InstalledAgent[]>('list_installed_agents').then(installed => {
      installedAgents.value = installed
      installedLoaded.value = true
    }).catch(err => {
      console.error('Failed to load installed status:', err)
      installedLoaded.value = true
    })
  } catch (error) {
    showMessage(`Failed to load agents: ${error}`, 'error')
  } finally {
    loading.value = false
  }
}

async function loadNativeConfig(agentId: string) {
  loading.value = true
  nativeConfig.value = null
  editing.value = false
  try {
    nativeConfig.value = await invoke<NativeConfig>('get_native_config', { agentId })
  } catch (error) {
    showMessage(`No config file found for ${agentId}`, 'error')
  } finally {
    loading.value = false
  }
}

function selectAgent(agent: AgentInfo) {
  selectedAgent.value = agent
  loadNativeConfig(agent.id)
}

function startEdit() {
  if (nativeConfig.value) {
    editContent.value = nativeConfig.value.config_content
    editing.value = true
  }
}

async function saveConfig() {
  if (!selectedAgent.value || !nativeConfig.value) return
  loading.value = true
  try {
    await invoke('save_native_config', {
      agentId: selectedAgent.value.id,
      content: editContent.value,
    })
    await loadNativeConfig(selectedAgent.value.id)
    editing.value = false
    showMessage('Config saved', 'success')
  } catch (error) {
    showMessage(`Failed to save config: ${error}`, 'error')
  } finally {
    loading.value = false
  }
}

function cancelEdit() {
  editing.value = false
  editContent.value = ''
  editKey.value = ''
  editValue.value = ''
}

function isInstalled(agentId: string): boolean {
  return installedAgents.value.find(a => a.id === agentId)?.installed ?? false
}

function getInstalledVersion(agentId: string): string | null {
  return installedAgents.value.find(a => a.id === agentId)?.version ?? null
}

const filteredAgents = computed(() => {
  let result = agents.value

  if (showInstalledOnly.value && installedLoaded.value) {
    result = result.filter(a => isInstalled(a.id))
  }

  if (searchQuery.value.trim()) {
    const q = searchQuery.value.toLowerCase()
    result = result.filter(a =>
      a.name.toLowerCase().includes(q) ||
      a.id.toLowerCase().includes(q) ||
      a.provider.toLowerCase().includes(q)
    )
  }

  return result
})

function showMessage(msg: string, type: 'success' | 'error') {
  message.value = msg
  messageType.value = type
  setTimeout(() => message.value = '', 3000)
}

function getValueType(value: any): string {
  if (value === null || value === undefined) return 'null'
  if (typeof value === 'boolean') return 'boolean'
  if (typeof value === 'number') return 'number'
  if (typeof value === 'string') return 'string'
  if (Array.isArray(value)) return 'array'
  if (typeof value === 'object') return 'object'
  return 'unknown'
}

function formatValue(value: any): string {
  if (value === null || value === undefined) return 'null'
  if (typeof value === 'boolean') return value ? 'true' : 'false'
  if (typeof value === 'number') return value.toString()
  if (typeof value === 'string') return value
  if (Array.isArray(value)) return JSON.stringify(value)
  if (typeof value === 'object') return JSON.stringify(value, null, 2)
  return String(value)
}

onMounted(loadAgents)
</script>

<template>
  <div class="config-manager">
    <header class="page-header">
      <h1>Configuration Manager</h1>
      <p>View and edit agent configuration files</p>
    </header>

    <div v-if="message" :class="['message', messageType]">
      {{ message }}
    </div>

    <div class="config-layout">
      <div class="config-sidebar">
        <div class="filter-controls">
          <input
            v-model="searchQuery"
            placeholder="Search agents..."
            class="search-input"
          />
          <label class="filter-toggle">
            <input type="checkbox" v-model="showInstalledOnly" />
            <span>Installed only</span>
          </label>
        </div>

        <div class="agent-list">
          <h3>Agents ({{ filteredAgents.length }})</h3>
          <div v-if="loading && agents.length === 0" class="loading">Loading...</div>
          <ul v-else>
            <li
              v-for="agent in filteredAgents"
              :key="agent.id"
              :class="{ active: selectedAgent?.id === agent.id, installed: isInstalled(agent.id) }"
              @click="selectAgent(agent)"
            >
              <div class="agent-info">
                <span class="agent-name">
                  {{ agent.name }}
                  <span v-if="isInstalled(agent.id)" class="installed-badge">✓</span>
                </span>
                <span class="agent-meta">
                  <span class="agent-provider">{{ agent.provider }}</span>
                  <span v-if="getInstalledVersion(agent.id)" class="version">
                    v{{ getInstalledVersion(agent.id) }}
                  </span>
                </span>
              </div>
              <span :class="['agent-type', agent.kind.toLowerCase()]">{{ agent.kind }}</span>
            </li>
          </ul>
          <p v-if="filteredAgents.length === 0 && !loading" class="empty">
            {{ showInstalledOnly ? 'No installed agents found' : 'No agents found' }}
          </p>
        </div>
      </div>

      <div class="config-detail">
        <div v-if="selectedAgent && nativeConfig" class="detail-content">
          <div class="detail-header">
            <h2>{{ selectedAgent.name }}</h2>
            <div class="detail-meta">
              <span class="badge">{{ selectedAgent.kind }}</span>
              <span class="badge">{{ selectedAgent.provider }}</span>
              <span class="badge format">{{ nativeConfig.config_format.toUpperCase() }}</span>
              <span v-if="isInstalled(selectedAgent.id)" class="badge installed">
                Installed {{ getInstalledVersion(selectedAgent.id) ? `v${getInstalledVersion(selectedAgent.id)}` : '' }}
              </span>
            </div>
            <div class="config-path">
              <span class="path-label">Path:</span>
              <code class="path-value">{{ nativeConfig.config_path }}</code>
            </div>
          </div>

          <!-- Parsed Config View -->
          <div v-if="nativeConfig.parsed && !editing" class="parsed-config">
            <div class="section-header">
              <h3>Configuration</h3>
              <button class="edit-btn" @click="startEdit">Edit Raw</button>
            </div>
            <div class="config-tree">
              <div v-for="(value, key) in nativeConfig.parsed" :key="key" class="config-item">
                <div class="config-key">{{ key }}</div>
                <div class="config-value">
                  <span v-if="getValueType(value) === 'boolean'" :class="['value-badge', value ? 'true' : 'false']">
                    {{ value ? '✓ true' : '✕ false' }}
                  </span>
                  <span v-else-if="getValueType(value) === 'number'" class="value-number">
                    {{ value }}
                  </span>
                  <span v-else-if="getValueType(value) === 'string'" class="value-string">
                    {{ value }}
                  </span>
                  <span v-else-if="getValueType(value) === 'null'" class="value-null">
                    null
                  </span>
                  <pre v-else class="value-complex">{{ formatValue(value) }}</pre>
                </div>
              </div>
            </div>
          </div>

          <!-- Raw Edit Mode -->
          <div v-else-if="editing" class="raw-editor">
            <div class="section-header">
              <h3>Edit Configuration</h3>
              <div class="editor-actions">
                <button class="save-btn" @click="saveConfig" :disabled="loading">Save</button>
                <button class="cancel-btn" @click="cancelEdit">Cancel</button>
              </div>
            </div>
            <textarea v-model="editContent" rows="30" class="raw-textarea" />
          </div>

          <!-- Fallback: Raw View -->
          <div v-else class="raw-view">
            <div class="section-header">
              <h3>Configuration (Raw)</h3>
              <button class="edit-btn" @click="startEdit">Edit</button>
            </div>
            <pre class="raw-content">{{ nativeConfig.config_content }}</pre>
          </div>
        </div>

        <div v-else-if="selectedAgent && loading" class="loading-state">
          <div class="spinner"></div>
          <p>Loading configuration...</p>
        </div>

        <div v-else class="no-selection">
          <div class="placeholder">
            <span class="placeholder-icon">⚙️</span>
            <p>Select an agent to view its configuration</p>
            <p class="hint">Select an installed agent to view its config</p>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.config-manager {
  padding: 2rem;
  height: 100%;
}

.page-header {
  margin-bottom: 2rem;
}

.page-header h1 {
  font-size: 1.75rem;
  color: #2c3e50;
  margin-bottom: 0.5rem;
}

.page-header p {
  color: #666;
}

.message {
  padding: 0.75rem 1rem;
  border-radius: 8px;
  margin-bottom: 1rem;
}

.message.success {
  background: #d4edda;
  color: #155724;
}

.message.error {
  background: #f8d7da;
  color: #721c24;
}

.config-layout {
  display: flex;
  gap: 2rem;
  height: calc(100vh - 200px);
}

.config-sidebar {
  width: 320px;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.filter-controls {
  background: white;
  padding: 1rem;
  border-radius: 12px;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.05);
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.search-input {
  width: 100%;
  padding: 0.6rem 1rem;
  border: 1px solid #e0e0e0;
  border-radius: 8px;
  font-size: 0.95rem;
}

.search-input:focus {
  outline: none;
  border-color: #3498db;
}

.filter-toggle {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  cursor: pointer;
  font-size: 0.9rem;
  color: #666;
}

.filter-toggle input {
  width: 16px;
  height: 16px;
}

.agent-list {
  background: white;
  padding: 1rem;
  border-radius: 12px;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.05);
  flex: 1;
  overflow-y: auto;
}

.agent-list h3 {
  margin-bottom: 0.75rem;
  color: #2c3e50;
  font-size: 1rem;
}

.agent-list ul {
  list-style: none;
}

.agent-list li {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0.75rem;
  border-radius: 8px;
  cursor: pointer;
  transition: background 0.2s;
  margin-bottom: 0.25rem;
}

.agent-list li:hover {
  background: #f8f9fa;
}

.agent-list li.active {
  background: #e3f2fd;
}

.agent-list li.installed {
  border-left: 3px solid #27ae60;
}

.agent-info {
  display: flex;
  flex-direction: column;
  gap: 0.2rem;
}

.agent-name {
  font-weight: 600;
  color: #2c3e50;
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.installed-badge {
  color: #27ae60;
  font-size: 0.9rem;
}

.agent-meta {
  display: flex;
  gap: 0.5rem;
  align-items: center;
}

.agent-provider {
  font-size: 0.8rem;
  color: #999;
}

.version {
  font-size: 0.75rem;
  color: #27ae60;
  background: #e8f5e9;
  padding: 0.1rem 0.4rem;
  border-radius: 4px;
}

.agent-type {
  padding: 0.2rem 0.5rem;
  border-radius: 4px;
  font-size: 0.75rem;
  font-weight: 600;
  text-transform: uppercase;
}

.agent-type.cli {
  background: #d4edda;
  color: #155724;
}

.agent-type.desktop {
  background: #e8daef;
  color: #6c3483;
}

.config-detail {
  flex: 1;
  background: white;
  padding: 2rem;
  border-radius: 12px;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.05);
  overflow-y: auto;
}

.detail-header {
  margin-bottom: 2rem;
  padding-bottom: 1.5rem;
  border-bottom: 1px solid #eee;
}

.detail-header h2 {
  color: #2c3e50;
  margin-bottom: 0.75rem;
}

.detail-meta {
  display: flex;
  gap: 0.5rem;
  flex-wrap: wrap;
  margin-bottom: 0.75rem;
}

.badge {
  padding: 0.3rem 0.75rem;
  background: #f0f0f0;
  color: #666;
  border-radius: 20px;
  font-size: 0.8rem;
}

.badge.format {
  background: #e3f2fd;
  color: #1976d2;
}

.badge.installed {
  background: #e8f5e9;
  color: #2e7d32;
}

.config-path {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.5rem 0.75rem;
  background: #f8f9fa;
  border-radius: 6px;
}

.path-label {
  font-size: 0.85rem;
  color: #666;
  font-weight: 500;
}

.path-value {
  font-size: 0.8rem;
  color: #1976d2;
  background: #e3f2fd;
  padding: 0.2rem 0.5rem;
  border-radius: 4px;
}

.section-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 1.5rem;
}

.section-header h3 {
  color: #2c3e50;
}

.edit-btn, .save-btn, .cancel-btn {
  padding: 0.5rem 1rem;
  border: none;
  border-radius: 6px;
  cursor: pointer;
  font-size: 0.9rem;
}

.edit-btn {
  background: #3498db;
  color: white;
}

.save-btn {
  background: #27ae60;
  color: white;
}

.cancel-btn {
  background: #95a5a6;
  color: white;
}

.editor-actions {
  display: flex;
  gap: 0.5rem;
}

.parsed-config {
  margin-top: 1rem;
}

.config-tree {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.config-item {
  display: flex;
  padding: 0.75rem;
  background: #f8f9fa;
  border-radius: 8px;
  gap: 1rem;
}

.config-key {
  font-weight: 600;
  color: #2c3e50;
  min-width: 200px;
  font-family: monospace;
}

.config-value {
  flex: 1;
  display: flex;
  align-items: center;
}

.value-badge {
  padding: 0.2rem 0.5rem;
  border-radius: 4px;
  font-size: 0.85rem;
  font-weight: 500;
}

.value-badge.true {
  background: #d4edda;
  color: #155724;
}

.value-badge.false {
  background: #f8d7da;
  color: #721c24;
}

.value-number {
  font-family: monospace;
  color: #1976d2;
  background: #e3f2fd;
  padding: 0.2rem 0.5rem;
  border-radius: 4px;
}

.value-string {
  color: #666;
  word-break: break-all;
}

.value-null {
  color: #999;
  font-style: italic;
}

.value-complex {
  background: #f8f9fa;
  padding: 0.5rem;
  border-radius: 4px;
  font-family: monospace;
  font-size: 0.85rem;
  overflow-x: auto;
  margin: 0;
  white-space: pre-wrap;
}

.raw-view, .raw-editor {
  margin-top: 1rem;
}

.raw-content {
  background: #f8f9fa;
  padding: 1rem;
  border-radius: 8px;
  font-family: monospace;
  font-size: 0.85rem;
  line-height: 1.5;
  overflow: auto;
  max-height: 500px;
  margin: 0;
}

.raw-textarea {
  width: 100%;
  padding: 1rem;
  border: 1px solid #e0e0e0;
  border-radius: 8px;
  font-family: monospace;
  font-size: 0.85rem;
  line-height: 1.5;
  resize: vertical;
}

.raw-textarea:focus {
  outline: none;
  border-color: #3498db;
}

.no-selection, .loading-state {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100%;
}

.placeholder {
  text-align: center;
  color: #999;
}

.placeholder-icon {
  font-size: 3rem;
  display: block;
  margin-bottom: 1rem;
}

.hint {
  font-size: 0.85rem;
  color: #bbb;
  margin-top: 0.5rem;
}

.spinner {
  width: 40px;
  height: 40px;
  border: 3px solid #f0f0f0;
  border-top-color: #3498db;
  border-radius: 50%;
  animation: spin 1s linear infinite;
  margin-bottom: 1rem;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.loading {
  text-align: center;
  padding: 2rem;
  color: #999;
}

.empty {
  text-align: center;
  padding: 2rem;
  color: #999;
  font-style: italic;
}
</style>
