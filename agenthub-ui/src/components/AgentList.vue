<script setup lang="ts">
import { ref, onMounted, computed, onUnmounted, shallowRef, onErrorCaptured } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import PageHeader from './common/PageHeader.vue'
import NotificationBar from './common/NotificationBar.vue'
import LoadingSpinner from './common/LoadingSpinner.vue'
import EmptyState from './common/EmptyState.vue'
import ModalDialog from './common/ModalDialog.vue'
import AgentToolbar from './agent/AgentToolbar.vue'
import BatchActions from './agent/BatchActions.vue'
import AgentCard from './agent/AgentCard.vue'
import AgentTable from './agent/AgentTable.vue'
import AgentDetailModalContent from './agent/AgentDetailModal.vue'

interface InstallerInfo {
  platform: string
  manager: string
  package: string | null
}

interface Agent {
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

interface InstallResult {
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

interface BatchResult {
  total: number
  success: number
  failed: number
  results: InstallResult[]
}

interface CacheEntry {
  data: Agent[]
  timestamp: number
}

const CACHE_KEY = 'agenthub_agents_cache'
const CACHE_TTL = 5 * 60 * 1000 // 5 minutes

const agents = shallowRef<Agent[]>([])
const searchQuery = ref('')
const loading = ref(false)
const message = ref('')
const messageType = ref<'success' | 'error'>('success')
const activeTab = ref<'all' | 'cli' | 'desktop'>('all')
const selectedAgents = ref<Set<string>>(new Set())
const installedMap = ref<Map<string, {installed: boolean, version: string | null}>>(new Map())
const cardProgress = ref<{[agentId: string]: {step: number, total_steps: number, message: string}}>({})
const batchProgress = ref<{current: number, total: number, agent: string, action: string} | null>(null)
const debouncedSearchQuery = ref('')
let searchTimeout: ReturnType<typeof setTimeout> | null = null
const viewMode = ref<'grid' | 'table'>('grid')
const sortBy = ref<'name' | 'type' | 'status'>('name')
const sortDirection = ref<'asc' | 'desc'>('asc')
const selectedAgent = ref<Agent | null>(null)
const showDetailModal = ref(false)
const lastRefresh = ref<number>(0)
const lastResults = ref<Record<string, InstallResult>>({})

function getCache(): CacheEntry | null {
  try {
    const cached = localStorage.getItem(CACHE_KEY)
    if (cached) {
      const entry: CacheEntry = JSON.parse(cached)
      if (Date.now() - entry.timestamp < CACHE_TTL) {
        return entry
      }
      localStorage.removeItem(CACHE_KEY)
    }
  } catch (e) {
    localStorage.removeItem(CACHE_KEY)
  }
  return null
}

function setCache(data: Agent[]) {
  try {
    const entry: CacheEntry = { data, timestamp: Date.now() }
    localStorage.setItem(CACHE_KEY, JSON.stringify(entry))
    lastRefresh.value = Date.now()
  } catch (e) {
    console.error('Failed to cache agents:', e)
  }
}

function clearCache() {
  localStorage.removeItem(CACHE_KEY)
  lastRefresh.value = 0
}

async function loadInstalledStatus() {
  try {
    const installed = await invoke<{id: string, installed: boolean, version: string | null}[]>('list_installed_agents')
    const map = new Map<string, {installed: boolean, version: string | null}>()
    installed.forEach(a => map.set(a.id, { installed: a.installed, version: a.version }))
    installedMap.value = map
  } catch (e) {
    console.error('Failed to load installed status:', e)
  }
}

const filteredAgents = computed(() => {
  let result = agents.value
  
  if (activeTab.value !== 'all') {
    result = result.filter(a => a.kind.toLowerCase() === activeTab.value)
  }
  
  if (debouncedSearchQuery.value.trim()) {
    const query = debouncedSearchQuery.value.toLowerCase()
    result = result.filter(a => 
      a.name.toLowerCase().includes(query) ||
      a.description.toLowerCase().includes(query) ||
      a.provider.toLowerCase().includes(query) ||
      a.id.toLowerCase().includes(query)
    )
  }
  
  result = [...result].sort((a, b) => {
    let comparison = 0
    switch (sortBy.value) {
      case 'name':
        comparison = a.name.localeCompare(b.name)
        break
      case 'type':
        comparison = a.kind.localeCompare(b.kind)
        break
      case 'status':
        comparison = a.status.localeCompare(b.status)
        break
    }
    return sortDirection.value === 'asc' ? comparison : -comparison
  })
  
  return result
})

const cliAgents = computed(() => agents.value.filter(a => a.kind === 'CLI'))
const desktopAgents = computed(() => agents.value.filter(a => a.kind === 'Desktop'))

async function loadAgents(forceRefresh = false) {
  if (!forceRefresh) {
    const cached = getCache()
    if (cached) {
      agents.value = cached.data
      return
    }
  }

  loading.value = true
  try {
    const [newAgents, _installed] = await Promise.all([
      invoke<Agent[]>('list_agents', { agentType: null }),
      loadInstalledStatus(),
    ])
    agents.value = newAgents
    setCache(newAgents)
  } catch (error) {
    console.error('Failed to load agents:', error)
  } finally {
    loading.value = false
  }
}

function debounceSearch() {
  if (searchTimeout) {
    clearTimeout(searchTimeout)
  }
  searchTimeout = setTimeout(() => {
    debouncedSearchQuery.value = searchQuery.value
  }, 300)
}

function toggleSort(field: 'name' | 'type' | 'status') {
  if (sortBy.value === field) {
    sortDirection.value = sortDirection.value === 'asc' ? 'desc' : 'asc'
  } else {
    sortBy.value = field
    sortDirection.value = 'asc'
  }
}

async function searchAgents() {
  if (!searchQuery.value.trim()) {
    await loadAgents()
    return
  }
  loading.value = true
  try {
    const agentType = activeTab.value === 'all' ? null : activeTab.value
    agents.value = await invoke('search_agents', { query: searchQuery.value, agentType })
  } catch (error) {
    console.error('Failed to search agents:', error)
  } finally {
    loading.value = false
  }
}

async function installAgent(name: string) {
  cardProgress.value[name] = { step: 1, total_steps: 3, message: 'Starting...' }
  delete lastResults.value[name]
  message.value = ''
  try {
    const result = await invoke<InstallResult>('install_agent', { name })
    if (result.success) {
      delete cardProgress.value[name]
      clearCache()
      await loadAgents(true)
      await loadInstalledStatus()
      message.value = `✅ ${name} installed (${result.duration_ms}ms)`
      messageType.value = 'success'
      setTimeout(() => message.value = '', 5000)
    } else {
      delete cardProgress.value[name]
      lastResults.value[name] = result
      const detail = result.stderr || result.stdout || result.message
      message.value = `❌ ${name}: ${detail}`
      messageType.value = 'error'
    }
  } catch (error) {
    delete cardProgress.value[name]
    message.value = `❌ Error: ${error}`
    messageType.value = 'error'
  }
}

async function uninstallAgent(name: string) {
  cardProgress.value[name] = { step: 1, total_steps: 3, message: 'Starting...' }
  delete lastResults.value[name]
  message.value = ''
  try {
    const result = await invoke<InstallResult>('uninstall_agent', { name })
    if (result.success) {
      delete cardProgress.value[name]
      clearCache()
      await loadAgents(true)
      await loadInstalledStatus()
      message.value = `✅ ${name} uninstalled (${result.duration_ms}ms)`
      messageType.value = 'success'
      setTimeout(() => message.value = '', 5000)
    } else {
      delete cardProgress.value[name]
      lastResults.value[name] = result
      const detail = result.stderr || result.stdout || result.message
      message.value = `❌ ${name}: ${detail}`
      messageType.value = 'error'
    }
  } catch (error) {
    delete cardProgress.value[name]
    message.value = `❌ Error: ${error}`
    messageType.value = 'error'
  }
}

async function cancelAgent(name: string) {
  try {
    await invoke<boolean>('cancel_operation', { name })
  } catch (error) {
    console.error('Failed to cancel operation:', error)
  }
}

function setTab(tab: 'all' | 'cli' | 'desktop') {
  activeTab.value = tab
  searchQuery.value = ''
  debouncedSearchQuery.value = ''
  selectedAgents.value.clear()
  loadAgents()
}

function toggleSelectAgent(name: string) {
  if (selectedAgents.value.has(name)) {
    selectedAgents.value.delete(name)
  } else {
    selectedAgents.value.add(name)
  }
}

function selectAllAgents() {
  const filtered = filteredAgents.value
  if (selectedAgents.value.size === filtered.length) {
    selectedAgents.value.clear()
  } else {
    filtered.forEach(agent => selectedAgents.value.add(agent.id))
  }
}

async function batchInstall() {
  if (selectedAgents.value.size === 0) {
    message.value = '❌ No agents selected'
    messageType.value = 'error'
    return
  }

  loading.value = true
  batchProgress.value = null
  message.value = ''
  try {
    const names = Array.from(selectedAgents.value)
    const result = await invoke<BatchResult>('batch_install_agents', { names })
    
    const failedResults = result.results.filter(r => !r.success)
    if (failedResults.length > 0) {
      const details = failedResults.map(r => `${r.agent_name}: ${r.stderr || r.message}`).join('; ')
      message.value = `❌ Batch install: ${result.success} succeeded, ${result.failed} failed — ${details}`
      messageType.value = 'error'
    } else {
      message.value = `✅ Batch install: ${result.success} agents installed successfully`
      messageType.value = 'success'
      selectedAgents.value.clear()
      clearCache()
      await loadAgents(true)
    }
  } catch (error) {
    message.value = `❌ Error: ${error}`
    messageType.value = 'error'
  } finally {
    loading.value = false
    batchProgress.value = null
  }
}

async function batchUninstall() {
  if (selectedAgents.value.size === 0) {
    message.value = '❌ No agents selected'
    messageType.value = 'error'
    return
  }

  loading.value = true
  batchProgress.value = null
  message.value = ''
  try {
    const names = Array.from(selectedAgents.value)
    const result = await invoke<BatchResult>('batch_uninstall_agents', { names })
    
    const failedResults = result.results.filter(r => !r.success)
    if (failedResults.length > 0) {
      const details = failedResults.map(r => `${r.agent_name}: ${r.stderr || r.message}`).join('; ')
      message.value = `❌ Batch uninstall: ${result.success} succeeded, ${result.failed} failed — ${details}`
      messageType.value = 'error'
    } else {
      message.value = `✅ Batch uninstall: ${result.success} agents uninstalled successfully`
      messageType.value = 'success'
      selectedAgents.value.clear()
      clearCache()
      await loadAgents(true)
    }
  } catch (error) {
    message.value = `❌ Error: ${error}`
    messageType.value = 'error'
  } finally {
    loading.value = false
    batchProgress.value = null
  }
}

let unlistenInstall: UnlistenFn | null = null
let unlistenUninstall: UnlistenFn | null = null
let unlistenBatch: UnlistenFn | null = null
let unlistenCancelled: UnlistenFn | null = null

onMounted(() => {
  loadAgents().then(() => loadInstalledStatus())
  listen('install-progress', (event) => {
    const p = event.payload as {name: string, step: number, total_steps: number, message: string}
    cardProgress.value[p.name] = { step: p.step, total_steps: p.total_steps, message: p.message }
  }).then(fn => unlistenInstall = fn)
  listen('uninstall-progress', (event) => {
    const p = event.payload as {name: string, step: number, total_steps: number, message: string}
    cardProgress.value[p.name] = { step: p.step, total_steps: p.total_steps, message: p.message }
  }).then(fn => unlistenUninstall = fn)
  listen('batch-progress', (event) => {
    batchProgress.value = event.payload as {current: number, total: number, agent: string, action: string}
  }).then(fn => unlistenBatch = fn)
  listen('operation-cancelled', (event) => {
    const p = event.payload as {name: string}
    delete cardProgress.value[p.name]
    delete lastResults.value[p.name]
    message.value = `✋ ${p.name} operation cancelled`
    messageType.value = 'success'
    setTimeout(() => message.value = '', 5000)
  }).then(fn => unlistenCancelled = fn)
})

onUnmounted(() => {
  batchProgress.value = null
  unlistenInstall?.()
  unlistenUninstall?.()
  unlistenBatch?.()
  unlistenCancelled?.()
  if (searchTimeout) {
    clearTimeout(searchTimeout)
  }
})

function openDetail(agent: Agent) {
  selectedAgent.value = agent
  showDetailModal.value = true
}

function closeDetail() {
  showDetailModal.value = false
  selectedAgent.value = null
}

onErrorCaptured((err, _instance, info) => {
  console.error('Error captured:', err, info)
  message.value = 'An error occurred: ' + (err as Error).message
  messageType.value = 'error'
  setTimeout(() => message.value = '', 5000)
  return false
})
</script>

<template>
  <div class="container">
    <PageHeader title="AgentHub" subtitle="Manage your AI coding agents" />
    <div class="agent-stats">
      <span class="stat-chip"><strong>{{ agents.length }}</strong> Total</span>
      <span class="stat-chip"><strong>{{ cliAgents.length }}</strong> CLI</span>
      <span class="stat-chip"><strong>{{ desktopAgents.length }}</strong> Desktop</span>
    </div>

    <NotificationBar :message="message" :type="messageType" @close="message = ''" />

    <div class="m3-tabs">
      <button 
        :class="['m3-tab', { active: activeTab === 'all' }]"
        @click="setTab('all')"
        @keydown.enter="setTab('all')"
        aria-label="Show all agents"
      >
        All Agents
        <span class="m3-tab-badge">{{ agents.length }}</span>
      </button>
      <button 
        :class="['m3-tab', { active: activeTab === 'cli' }]"
        @click="setTab('cli')"
        @keydown.enter="setTab('cli')"
        aria-label="Show CLI agents only"
      >
        CLI Agents
        <span class="m3-tab-badge">{{ cliAgents.length }}</span>
      </button>
      <button 
        :class="['m3-tab', { active: activeTab === 'desktop' }]"
        @click="setTab('desktop')"
        @keydown.enter="setTab('desktop')"
        aria-label="Show desktop agents only"
      >
        Desktop Agents
        <span class="m3-tab-badge">{{ desktopAgents.length }}</span>
      </button>
    </div>

    <AgentToolbar
      :search-query="searchQuery"
      :view-mode="viewMode"
      :sort-by="sortBy"
      :sort-direction="sortDirection"
      :loading="loading"
      :active-tab="activeTab"
      @search-update="searchQuery = $event; debounceSearch()"
      @search="searchAgents"
      @refresh="loadAgents(true)"
      @toggle-sort="toggleSort"
      @toggle-view="viewMode = $event"
      @set-tab="setTab"
    />

    <!-- Batch Progress Overlay -->
    <div v-if="batchProgress" class="progress-overlay">
      <div class="progress-card">
        <div class="progress-header">
          <span class="progress-title">{{ batchProgress.action === 'install' ? 'Batch Install' : 'Batch Uninstall' }}</span>
          <span class="progress-badge badge-running">{{ batchProgress.current }}/{{ batchProgress.total }}</span>
        </div>
        <div class="progress-bar-track">
          <div class="progress-fill" :style="{ width: (batchProgress.current / batchProgress.total * 100) + '%' }"></div>
        </div>
        <div class="progress-message">Processing: {{ batchProgress.agent }}</div>
        <button class="m3-btn-outlined" @click="cancelAgent(batchProgress.agent)" style="margin-top: 1rem;">Cancel</button>
      </div>
    </div>

    <!-- Initial Loading Spinner (first load only) -->
    <LoadingSpinner v-if="loading && agents.length === 0" text="Loading agents..." />

    <BatchActions
      :count="selectedAgents.size"
      :total="filteredAgents.length"
      :loading="loading"
      @select-all="selectAllAgents"
      @batch-install="batchInstall"
      @batch-uninstall="batchUninstall"
    />

    <!-- Grid View -->
    <div v-if="viewMode === 'grid' && !loading && filteredAgents.length > 0" class="agents-grid">
      <AgentCard
        v-for="agent in filteredAgents"
        :key="agent.id"
        :agent="agent"
        :is-selected="selectedAgents.has(agent.id)"
        :installed="!!installedMap.get(agent.id)?.installed"
        :version="installedMap.get(agent.id)?.version || null"
        :progress="cardProgress[agent.id] || null"
        :result="lastResults[agent.id] || null"
        @toggle-select="toggleSelectAgent"
        @open-detail="openDetail"
        @install="installAgent"
        @uninstall="uninstallAgent"
        @cancel="cancelAgent"
      />
    </div>

    <!-- Table View -->
    <AgentTable
      v-if="viewMode === 'table' && !loading && filteredAgents.length > 0"
      :agents="filteredAgents"
      :selected-agents="selectedAgents"
      :sort-by="sortBy"
      :sort-direction="sortDirection"
      :loading="loading"
      :progress="cardProgress"
      :installed-map="installedMap"
      :results="lastResults"
      @toggle-sort="toggleSort"
      @toggle-select="toggleSelectAgent"
      @select-all="selectAllAgents"
      @install="installAgent"
      @uninstall="uninstallAgent"
      @cancel="cancelAgent"
    />

    <EmptyState v-if="!loading && filteredAgents.length === 0" text="No agents found" />

    <!-- Detail Modal -->
    <ModalDialog :show="showDetailModal" :title="selectedAgent?.name" @close="closeDetail">
      <AgentDetailModalContent
        v-if="selectedAgent"
        :agent="selectedAgent"
        :loading="loading"
        @install="installAgent(selectedAgent.id); closeDetail()"
        @uninstall="uninstallAgent(selectedAgent.id); closeDetail()"
        @close="closeDetail"
      />
    </ModalDialog>
  </div>
</template>

<style scoped>
.container {
  width: 100%;
  max-width: 100%;
  margin: 0;
  padding: 2rem;
  min-height: 100vh;
  background: var(--md-sys-color-background);
  box-sizing: border-box;
}

.m3-tab-badge {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 1.25rem;
  height: 1.25rem;
  padding: 0 0.375rem;
  border-radius: var(--md-sys-shape-full);
  background: var(--md-sys-color-surface-variant);
  color: var(--md-sys-color-on-surface-variant);
  font: var(--md-sys-typescale-label-small);
}
.m3-tab.active .m3-tab-badge {
  background: var(--md-sys-color-primary-container);
  color: var(--md-sys-color-on-primary-container);
}

.agents-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(340px, 1fr));
  gap: 1rem;
}

/* Progress Overlay */
.progress-overlay {
  position: fixed; top: 0; left: 0; right: 0; bottom: 0;
  background: rgba(0,0,0,0.38); display: flex;
  align-items: center; justify-content: center; z-index: 900;
}
.progress-card {
  background: var(--md-sys-color-surface);
  padding: 2rem; border-radius: var(--md-sys-shape-lg);
  min-width: 320px; box-shadow: var(--md-sys-elevation-3);
}
.progress-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 1rem; }
.progress-title { font: var(--md-sys-typescale-title-medium); color: var(--md-sys-color-on-surface); }
.progress-badge { padding: 0.2rem 0.6rem; border-radius: var(--md-sys-shape-full); font: var(--md-sys-typescale-label-small); }
.badge-running { background: var(--md-sys-color-secondary-container); color: var(--md-sys-color-on-secondary-container); }
.progress-bar-track { height: 6px; background: var(--md-sys-color-surface-variant); border-radius: var(--md-sys-shape-full); overflow: hidden; margin-bottom: 0.75rem; }
.progress-fill { height: 100%; background: var(--md-sys-color-primary); border-radius: var(--md-sys-shape-full); transition: width 0.3s ease; }
.progress-message { font: var(--md-sys-typescale-body-small); color: var(--md-sys-color-on-surface-variant); text-align: center; }

button:disabled { opacity: 0.38; cursor: not-allowed; }

@media (min-width: 1200px) {
  .agents-grid { grid-template-columns: repeat(auto-fill, minmax(360px, 1fr)); }
}
@media (min-width: 1600px) {
  .agents-grid { grid-template-columns: repeat(4, 1fr); }
}
@media (max-width: 992px) {
  .container { padding: 1.5rem; }
  .agent-stats { justify-content: center; }
  .m3-tabs { flex-wrap: wrap; }
  .m3-tab { flex: 1 1 calc(50% - 0.25rem); }
}
@media (max-width: 768px) {
  .agents-grid { grid-template-columns: 1fr; }
}
@media (max-width: 480px) {
  .container { padding: 1rem; }
}
</style>
