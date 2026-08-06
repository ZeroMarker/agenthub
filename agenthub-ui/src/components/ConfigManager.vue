<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import PageHeader from './common/PageHeader.vue'
import NotificationBar from './common/NotificationBar.vue'
import LoadingSpinner from './common/LoadingSpinner.vue'
import EmptyState from './common/EmptyState.vue'

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
    <PageHeader title="Configuration Manager" subtitle="View and edit agent configuration files" />

    <NotificationBar :message="message" :type="messageType" @close="message = ''" />

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
          <LoadingSpinner v-if="loading && agents.length === 0" />
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
          <EmptyState v-if="filteredAgents.length === 0 && !loading" :text="showInstalledOnly ? 'No installed agents found' : 'No agents found'" />
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
  background: var(--md-sys-color-surface);
  padding: 1rem;
  border-radius: var(--md-sys-shape-md);
  box-shadow: var(--md-sys-elevation-1);
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.search-input {
  width: 100%;
  padding: 0.6rem 1rem;
  border: 1px solid var(--md-sys-color-outline-variant);
  border-radius: var(--md-sys-shape-sm);
  font-size: 0.95rem;
}

.search-input:focus {
  outline: none;
  border-color: var(--md-sys-color-primary);
}

.filter-toggle {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  cursor: pointer;
  font-size: 0.9rem;
  color: var(--md-sys-color-on-surface-variant);
}

.filter-toggle input {
  width: 16px;
  height: 16px;
}

.agent-list {
  background: var(--md-sys-color-surface);
  padding: 1rem;
  border-radius: var(--md-sys-shape-md);
  box-shadow: var(--md-sys-elevation-1);
  flex: 1;
  overflow-y: auto;
}

.agent-list h3 {
  margin-bottom: 0.75rem;
  color: var(--md-sys-color-on-surface);
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
  border-radius: var(--md-sys-shape-sm);
  cursor: pointer;
  transition: background 0.2s;
  margin-bottom: 0.25rem;
}

.agent-list li:hover {
  background: var(--md-sys-color-surface-variant);
}

.agent-list li.active {
  background: var(--md-sys-color-primary-container);
}

.agent-list li.installed {
  border-left: 3px solid var(--md-sys-color-primary);
}

.agent-info {
  display: flex;
  flex-direction: column;
  gap: 0.2rem;
}

.agent-name {
  font-weight: 600;
  color: var(--md-sys-color-on-surface);
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.installed-badge {
  color: var(--md-sys-color-primary);
  font-size: 0.9rem;
}

.agent-meta {
  display: flex;
  gap: 0.5rem;
  align-items: center;
}

.agent-provider {
  font-size: 0.8rem;
  color: var(--md-sys-color-on-surface-variant);
}

.version {
  font-size: 0.75rem;
  color: var(--md-sys-color-primary);
  background: var(--md-sys-color-secondary-container);
  padding: 0.1rem 0.4rem;
  border-radius: var(--md-sys-shape-xs);
}

.agent-type {
  padding: 0.2rem 0.5rem;
  border-radius: var(--md-sys-shape-xs);
  font-size: 0.75rem;
  font-weight: 600;
  text-transform: uppercase;
}

.agent-type.cli {
  background: var(--md-sys-color-secondary-container);
  color: var(--md-sys-color-on-secondary-container);
}

.agent-type.desktop {
  background: var(--md-sys-color-tertiary-container);
  color: var(--md-sys-color-on-tertiary-container);
}

.config-detail {
  flex: 1;
  background: var(--md-sys-color-surface);
  padding: 2rem;
  border-radius: var(--md-sys-shape-md);
  box-shadow: var(--md-sys-elevation-1);
  overflow-y: auto;
}

.detail-header {
  margin-bottom: 2rem;
  padding-bottom: 1.5rem;
  border-bottom: 1px solid var(--md-sys-color-outline-variant);
}

.detail-header h2 {
  color: var(--md-sys-color-on-surface);
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
  background: var(--md-sys-color-surface-variant);
  color: var(--md-sys-color-on-surface-variant);
  border-radius: var(--md-sys-shape-xl);
  font-size: 0.8rem;
}

.badge.format {
  background: var(--md-sys-color-primary-container);
  color: var(--md-sys-color-primary);
}

.badge.installed {
  background: var(--md-sys-color-secondary-container);
  color: var(--md-sys-color-primary);
}

.config-path {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.5rem 0.75rem;
  background: var(--md-sys-color-surface-variant);
  border-radius: var(--md-sys-shape-xs);
}

.path-label {
  font-size: 0.85rem;
  color: var(--md-sys-color-on-surface-variant);
  font-weight: 500;
}

.path-value {
  font-size: 0.8rem;
  color: var(--md-sys-color-primary);
  background: var(--md-sys-color-primary-container);
  padding: 0.2rem 0.5rem;
  border-radius: var(--md-sys-shape-xs);
}

.section-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 1.5rem;
}

.section-header h3 {
  color: var(--md-sys-color-on-surface);
}

.edit-btn, .save-btn, .cancel-btn {
  padding: 0.5rem 1rem;
  border: none;
  border-radius: var(--md-sys-shape-xs);
  cursor: pointer;
  font-size: 0.9rem;
}

.edit-btn {
  background: var(--md-sys-color-primary);
  color: var(--md-sys-color-on-primary);
}

.save-btn {
  background: var(--md-sys-color-primary);
  color: var(--md-sys-color-on-primary);
}

.cancel-btn {
  background: var(--md-sys-color-surface-variant);
  color: var(--md-sys-color-on-surface);
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
  background: var(--md-sys-color-surface-variant);
  border-radius: var(--md-sys-shape-sm);
  gap: 1rem;
}

.config-key {
  font-weight: 600;
  color: var(--md-sys-color-on-surface);
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
  border-radius: var(--md-sys-shape-xs);
  font-size: 0.85rem;
  font-weight: 500;
}

.value-badge.true {
  background: var(--md-sys-color-secondary-container);
  color: var(--md-sys-color-on-secondary-container);
}

.value-badge.false {
  background: var(--md-sys-color-error-container);
  color: var(--md-sys-color-on-error-container);
}

.value-number {
  font-family: monospace;
  color: var(--md-sys-color-primary);
  background: var(--md-sys-color-primary-container);
  padding: 0.2rem 0.5rem;
  border-radius: var(--md-sys-shape-xs);
}

.value-string {
  color: var(--md-sys-color-on-surface-variant);
  word-break: break-all;
}

.value-null {
  color: var(--md-sys-color-on-surface-variant);
  font-style: italic;
}

.value-complex {
  background: var(--md-sys-color-surface-variant);
  padding: 0.5rem;
  border-radius: var(--md-sys-shape-xs);
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
  background: var(--md-sys-color-surface-variant);
  padding: 1rem;
  border-radius: var(--md-sys-shape-sm);
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
  border: 1px solid var(--md-sys-color-outline-variant);
  border-radius: var(--md-sys-shape-sm);
  font-family: monospace;
  font-size: 0.85rem;
  line-height: 1.5;
  resize: vertical;
}

.raw-textarea:focus {
  outline: none;
  border-color: var(--md-sys-color-primary);
}

.no-selection, .loading-state {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100%;
}

.placeholder {
  text-align: center;
  color: var(--md-sys-color-on-surface-variant);
}

.placeholder-icon {
  font-size: 3rem;
  display: block;
  margin-bottom: 1rem;
}

.hint {
  font-size: 0.85rem;
  color: var(--md-sys-color-on-surface-variant);
  opacity: 0.7;
  margin-top: 0.5rem;
}

/* Responsive */
@media (max-width: 1200px) {
  .config-layout { flex-direction: column; height: auto; }
  .config-sidebar { width: 100%; }
  .config-detail { min-height: 400px; }
}
@media (max-width: 900px) {
  .config-manager { padding: 1.25rem; }
  .filter-controls { padding: 0.75rem; }
  .config-detail { padding: 1.25rem; }
}
@media (max-width: 600px) {
  .config-manager { padding: 1rem; }
  .config-detail { padding: 1rem; }
  .config-item { flex-direction: column; gap: 0.5rem; }
  .config-key { min-width: auto; }
  .detail-meta { flex-wrap: wrap; }
}
</style>
