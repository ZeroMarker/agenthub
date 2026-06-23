<script setup lang="ts">
import { ref, onMounted, computed, onUnmounted, shallowRef, onErrorCaptured } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

interface Agent {
  name: string
  description: string
  package_name: string
  manager: string
  agent_type: 'CLI' | 'Desktop'
  install_source: string
  download_url: string
  version: string | null
  installed: boolean
}

interface InstallResult {
  success: boolean
  message: string
  agent_name: string
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
const progress = ref<{name: string, step: number, total_steps: number, message: string} | null>(null)
const batchProgress = ref<{current: number, total: number, agent: string, action: string} | null>(null)
const debouncedSearchQuery = ref('')
let searchTimeout: ReturnType<typeof setTimeout> | null = null
const viewMode = ref<'grid' | 'table'>('grid')
const sortBy = ref<'name' | 'type' | 'status'>('name')
const sortDirection = ref<'asc' | 'desc'>('asc')
const selectedAgent = ref<Agent | null>(null)
const showDetailModal = ref(false)
const lastRefresh = ref<number>(0)

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

const filteredAgents = computed(() => {
  let result = agents.value
  
  if (activeTab.value !== 'all') {
    result = result.filter(a => a.agent_type.toLowerCase() === activeTab.value)
  }
  
  if (debouncedSearchQuery.value.trim()) {
    const query = debouncedSearchQuery.value.toLowerCase()
    result = result.filter(a => 
      a.name.toLowerCase().includes(query) ||
      a.description.toLowerCase().includes(query) ||
      a.package_name.toLowerCase().includes(query)
    )
  }
  
  result = [...result].sort((a, b) => {
    let comparison = 0
    switch (sortBy.value) {
      case 'name':
        comparison = a.name.localeCompare(b.name)
        break
      case 'type':
        comparison = a.agent_type.localeCompare(b.agent_type)
        break
      case 'status':
        comparison = (a.installed ? 1 : 0) - (b.installed ? 1 : 0)
        break
    }
    return sortDirection.value === 'asc' ? comparison : -comparison
  })
  
  return result
})

const cliAgents = computed(() => agents.value.filter(a => a.agent_type === 'CLI'))
const desktopAgents = computed(() => agents.value.filter(a => a.agent_type === 'Desktop'))
const installedAgents = computed(() => agents.value.filter(a => a.installed))
const notInstalledAgents = computed(() => agents.value.filter(a => !a.installed))

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
    const newAgents = await invoke<Agent[]>('list_agents', { agentType: null })
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

function getSortIcon(field: 'name' | 'type' | 'status') {
  if (sortBy.value !== field) return '↕'
  return sortDirection.value === 'asc' ? '↑' : '↓'
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
  loading.value = true
  progress.value = null
  try {
    const result = await invoke<InstallResult>('install_agent', { name })
    if (result.success) {
      message.value = result.message
      messageType.value = 'success'
    } else {
      message.value = result.message
      messageType.value = 'error'
    }
    clearCache()
    await loadAgents(true)
  } catch (error) {
    message.value = error as string
    messageType.value = 'error'
  } finally {
    loading.value = false
    progress.value = null
    setTimeout(() => message.value = '', 3000)
  }
}

async function uninstallAgent(name: string) {
  loading.value = true
  progress.value = null
  try {
    const result = await invoke<InstallResult>('uninstall_agent', { name })
    if (result.success) {
      message.value = result.message
      messageType.value = 'success'
    } else {
      message.value = result.message
      messageType.value = 'error'
    }
    clearCache()
    await loadAgents(true)
  } catch (error) {
    message.value = error as string
    messageType.value = 'error'
  } finally {
    loading.value = false
    progress.value = null
    setTimeout(() => message.value = '', 3000)
  }
}

function setTab(tab: 'all' | 'cli' | 'desktop') {
  activeTab.value = tab
  searchQuery.value = ''
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
    filtered.forEach(agent => selectedAgents.value.add(agent.name))
  }
}

async function batchInstall() {
  if (selectedAgents.value.size === 0) {
    message.value = 'No agents selected'
    messageType.value = 'error'
    setTimeout(() => message.value = '', 3000)
    return
  }

  loading.value = true
  batchProgress.value = null
  try {
    const names = Array.from(selectedAgents.value)
    const result = await invoke<BatchResult>('batch_install_agents', { names })
    
    const failedResults = result.results.filter(r => !r.success)
    if (failedResults.length > 0) {
      message.value = `Batch install: ${result.success} succeeded, ${result.failed} failed`
      messageType.value = 'error'
    } else {
      message.value = `Batch install: ${result.success} agents installed successfully`
      messageType.value = 'success'
    }
    
    selectedAgents.value.clear()
    clearCache()
    await loadAgents(true)
  } catch (error) {
    message.value = error as string
    messageType.value = 'error'
  } finally {
    loading.value = false
    batchProgress.value = null
    setTimeout(() => message.value = '', 5000)
  }
}

async function batchUninstall() {
  if (selectedAgents.value.size === 0) {
    message.value = 'No agents selected'
    messageType.value = 'error'
    setTimeout(() => message.value = '', 3000)
    return
  }

  loading.value = true
  batchProgress.value = null
  try {
    const names = Array.from(selectedAgents.value)
    const result = await invoke<BatchResult>('batch_uninstall_agents', { names })
    
    const failedResults = result.results.filter(r => !r.success)
    if (failedResults.length > 0) {
      message.value = `Batch uninstall: ${result.success} succeeded, ${result.failed} failed`
      messageType.value = 'error'
    } else {
      message.value = `Batch uninstall: ${result.success} agents uninstalled successfully`
      messageType.value = 'success'
    }
    
    selectedAgents.value.clear()
    clearCache()
    await loadAgents(true)
  } catch (error) {
    message.value = error as string
    messageType.value = 'error'
  } finally {
    loading.value = false
    batchProgress.value = null
    setTimeout(() => message.value = '', 5000)
  }
}

onMounted(async () => {
  await loadAgents()
  
  // Listen for install progress events
  await listen('install-progress', (event) => {
    progress.value = event.payload as {name: string, step: number, total_steps: number, message: string}
  })
  
  // Listen for uninstall progress events
  await listen('uninstall-progress', (event) => {
    progress.value = event.payload as {name: string, step: number, total_steps: number, message: string}
  })
  
  // Listen for batch progress events
  await listen('batch-progress', (event) => {
    batchProgress.value = event.payload as {current: number, total: number, agent: string, action: string}
  })
})

onUnmounted(() => {
  progress.value = null
  batchProgress.value = null
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
    <header>
      <div class="header-content">
        <div class="header-title">
          <h1>AgentHub</h1>
          <p>Manage your AI coding agents</p>
        </div>
        <div class="header-stats">
          <div class="stat-item">
            <span class="stat-value">{{ agents.length }}</span>
            <span class="stat-label">Total</span>
          </div>
          <div class="stat-item">
            <span class="stat-value">{{ installedAgents.length }}</span>
            <span class="stat-label">Installed</span>
          </div>
          <div class="stat-item">
            <span class="stat-value">{{ notInstalledAgents.length }}</span>
            <span class="stat-label">Available</span>
          </div>
        </div>
      </div>
    </header>

    <Transition name="fade">
      <div v-if="message" :class="['message', messageType]">
        <span class="message-icon">{{ messageType === 'success' ? '✓' : '✕' }}</span>
        {{ message }}
      </div>
    </Transition>

    <div class="tabs">
      <button 
        :class="['tab', { active: activeTab === 'all' }]"
        @click="setTab('all')"
      >
        All Agents
        <span class="badge">{{ agents.length }}</span>
      </button>
      <button 
        :class="['tab', { active: activeTab === 'cli' }]"
        @click="setTab('cli')"
      >
        CLI Agents
        <span class="badge">{{ cliAgents.length }}</span>
      </button>
      <button 
        :class="['tab', { active: activeTab === 'desktop' }]"
        @click="setTab('desktop')"
      >
        Desktop Agents
        <span class="badge">{{ desktopAgents.length }}</span>
      </button>
    </div>

    <div class="toolbar">
      <div class="search-bar">
        <input
          v-model="searchQuery"
          type="text"
          :placeholder="`Search ${activeTab === 'all' ? 'all' : activeTab} agents...`"
          @input="debounceSearch"
          @keyup.enter="searchAgents"
        />
        <button @click="searchAgents" :disabled="loading">
          Search
        </button>
        <button @click="loadAgents(true)" :disabled="loading" class="refresh-btn">
          Refresh
        </button>
      </div>
      <div class="view-controls">
        <div class="sort-controls">
          <button @click="toggleSort('name')" :class="['sort-btn', { active: sortBy === 'name' }]">
            Name {{ getSortIcon('name') }}
          </button>
          <button @click="toggleSort('type')" :class="['sort-btn', { active: sortBy === 'type' }]">
            Type {{ getSortIcon('type') }}
          </button>
          <button @click="toggleSort('status')" :class="['sort-btn', { active: sortBy === 'status' }]">
            Status {{ getSortIcon('status') }}
          </button>
        </div>
        <div class="view-toggle">
          <button @click="viewMode = 'grid'" :class="['view-btn', { active: viewMode === 'grid' }]">
            ⊞ Grid
          </button>
          <button @click="viewMode = 'table'" :class="['view-btn', { active: viewMode === 'table' }]">
            ☰ Table
          </button>
        </div>
      </div>
    </div>

    <div v-if="loading" class="loading">
      <div v-if="progress" class="progress-container">
        <div class="progress-info">
          <span class="progress-name">{{ progress.name }}</span>
          <span class="progress-step">Step {{ progress.step }}/{{ progress.total_steps }}</span>
        </div>
        <div class="progress-bar">
          <div 
            class="progress-fill" 
            :style="{ width: (progress.step / progress.total_steps * 100) + '%' }"
          ></div>
        </div>
        <div class="progress-message">{{ progress.message }}</div>
      </div>
      <div v-else-if="batchProgress" class="progress-container">
        <div class="progress-info">
          <span class="progress-name">{{ batchProgress.action === 'install' ? 'Installing' : 'Uninstalling' }}: {{ batchProgress.agent }}</span>
          <span class="progress-step">{{ batchProgress.current }}/{{ batchProgress.total }}</span>
        </div>
        <div class="progress-bar">
          <div 
            class="progress-fill" 
            :style="{ width: (batchProgress.current / batchProgress.total * 100) + '%' }"
          ></div>
        </div>
        <div class="progress-message">Processing batch {{ batchProgress.action }}...</div>
      </div>
      <div v-else class="loading-spinner">
        <div class="spinner"></div>
        <div class="loading-text">Loading agents...</div>
      </div>
    </div>

    <div v-if="!loading && filteredAgents.length > 0" class="batch-actions">
      <div class="select-all">
        <input 
          type="checkbox" 
          :checked="selectedAgents.size === filteredAgents.length && filteredAgents.length > 0"
          @change="selectAllAgents"
        />
        <span>Select all ({{ selectedAgents.size }}/{{ filteredAgents.length }})</span>
      </div>
      <div class="batch-buttons">
        <button 
          @click="batchInstall" 
          :disabled="loading || selectedAgents.size === 0"
          class="batch-install-btn"
        >
          Install Selected ({{ selectedAgents.size }})
        </button>
        <button 
          @click="batchUninstall" 
          :disabled="loading || selectedAgents.size === 0"
          class="batch-uninstall-btn"
        >
          Uninstall Selected ({{ selectedAgents.size }})
        </button>
      </div>
    </div>

    <!-- Grid View -->
    <div v-if="viewMode === 'grid' && !loading && filteredAgents.length > 0" class="agents-grid">
      <div v-for="agent in filteredAgents" :key="agent.name" :class="['agent-card', agent.agent_type.toLowerCase()]" @click="openDetail(agent)">
        <div class="agent-header">
          <div class="agent-title">
            <input 
              type="checkbox" 
              :checked="selectedAgents.has(agent.name)"
              @change="toggleSelectAgent(agent.name)"
              @click.stop
            />
            <h3>{{ agent.name }}</h3>
          </div>
          <div class="badges">
            <span :class="['type-badge', agent.agent_type.toLowerCase()]">
              {{ agent.agent_type }}
            </span>
            <span :class="['status', agent.installed ? 'installed' : 'not-installed']">
              {{ agent.installed ? 'Installed' : 'Not Installed' }}
            </span>
          </div>
        </div>
        <p class="description">{{ agent.description }}</p>
        <div class="agent-meta">
          <span class="package">{{ agent.package_name }}</span>
          <span class="manager">{{ agent.manager }}</span>
        </div>
        <div class="actions" @click.stop>
          <button
            v-if="!agent.installed"
            @click="installAgent(agent.name)"
            :disabled="loading"
            class="install-btn"
          >
            Install
          </button>
          <button
            v-else
            @click="uninstallAgent(agent.name)"
            :disabled="loading"
            class="uninstall-btn"
          >
            Uninstall
          </button>
        </div>
      </div>
    </div>

    <!-- Table View -->
    <div v-if="viewMode === 'table' && !loading && filteredAgents.length > 0" class="agents-table-container">
      <table class="agents-table">
        <thead>
          <tr>
            <th class="checkbox-col">
              <input 
                type="checkbox" 
                :checked="selectedAgents.size === filteredAgents.length && filteredAgents.length > 0"
                @change="selectAllAgents"
              />
            </th>
            <th @click="toggleSort('name')" class="sortable">
              Name {{ getSortIcon('name') }}
            </th>
            <th @click="toggleSort('type')" class="sortable">
              Type {{ getSortIcon('type') }}
            </th>
            <th>Description</th>
            <th>Package</th>
            <th>Manager</th>
            <th>Source</th>
            <th @click="toggleSort('status')" class="sortable">
              Status {{ getSortIcon('status') }}
            </th>
            <th>Actions</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="agent in filteredAgents" :key="agent.name" :class="['agent-row', agent.agent_type.toLowerCase()]">
            <td class="checkbox-col">
              <input 
                type="checkbox" 
                :checked="selectedAgents.has(agent.name)"
                @change="toggleSelectAgent(agent.name)"
              />
            </td>
            <td class="name-col">
              <span class="agent-name">{{ agent.name }}</span>
            </td>
            <td>
              <span :class="['type-badge', agent.agent_type.toLowerCase()]">
                {{ agent.agent_type }}
              </span>
            </td>
            <td class="description-col">{{ agent.description }}</td>
            <td class="package-col">
              <code>{{ agent.package_name }}</code>
            </td>
            <td>
              <span class="manager-badge">{{ agent.manager }}</span>
            </td>
            <td class="source-col">{{ agent.install_source }}</td>
            <td>
              <span :class="['status-badge', agent.installed ? 'installed' : 'not-installed']">
                {{ agent.installed ? '✓ Installed' : '○ Not Installed' }}
              </span>
            </td>
            <td class="actions-col">
              <button
                v-if="!agent.installed"
                @click="installAgent(agent.name)"
                :disabled="loading"
                class="install-btn-sm"
              >
                Install
              </button>
              <button
                v-else
                @click="uninstallAgent(agent.name)"
                :disabled="loading"
                class="uninstall-btn-sm"
              >
                Uninstall
              </button>
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <div v-if="!loading && filteredAgents.length === 0" class="empty">
      No agents found
    </div>

    <!-- Detail Modal -->
    <Teleport to="body">
      <div v-if="showDetailModal && selectedAgent" class="modal-overlay" @click.self="closeDetail">
        <div class="modal-content">
          <div class="modal-header">
            <div class="modal-title">
              <h2>{{ selectedAgent.name }}</h2>
              <span :class="['type-badge', selectedAgent.agent_type.toLowerCase()]">
                {{ selectedAgent.agent_type }}
              </span>
              <span :class="['status-badge', selectedAgent.installed ? 'installed' : 'not-installed']">
                {{ selectedAgent.installed ? '✓ Installed' : '○ Not Installed' }}
              </span>
            </div>
            <button class="modal-close" @click="closeDetail">&times;</button>
          </div>
          <div class="modal-body">
            <p class="modal-description">{{ selectedAgent.description }}</p>
            
            <div class="detail-grid">
              <div class="detail-item">
                <span class="detail-label">Package</span>
                <code class="detail-value">{{ selectedAgent.package_name }}</code>
              </div>
              <div class="detail-item">
                <span class="detail-label">Manager</span>
                <span class="detail-value manager-badge">{{ selectedAgent.manager }}</span>
              </div>
              <div class="detail-item">
                <span class="detail-label">Install Source</span>
                <span class="detail-value">{{ selectedAgent.install_source }}</span>
              </div>
              <div v-if="selectedAgent.download_url" class="detail-item">
                <span class="detail-label">Website</span>
                <a :href="'https://' + selectedAgent.download_url" target="_blank" class="detail-value link">
                  {{ selectedAgent.download_url }}
                </a>
              </div>
              <div v-if="selectedAgent.version" class="detail-item">
                <span class="detail-label">Version</span>
                <span class="detail-value">{{ selectedAgent.version }}</span>
              </div>
            </div>
          </div>
          <div class="modal-footer">
            <button
              v-if="!selectedAgent.installed"
              @click="installAgent(selectedAgent.name); closeDetail()"
              :disabled="loading"
              class="modal-install-btn"
            >
              Install
            </button>
            <button
              v-else
              @click="uninstallAgent(selectedAgent.name); closeDetail()"
              :disabled="loading"
              class="modal-uninstall-btn"
            >
              Uninstall
            </button>
            <button @click="closeDetail" class="modal-cancel-btn">
              Close
            </button>
          </div>
        </div>
      </div>
    </Teleport>
  </div>
</template>

<style scoped>
.container {
  width: 100%;
  max-width: 100%;
  margin: 0;
  padding: 2rem;
  min-height: 100vh;
  background: linear-gradient(135deg, #f5f7fa 0%, #c3cfe2 100%);
  box-sizing: border-box;
}

header {
  margin-bottom: 2rem;
  padding: 2rem;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  border-radius: 16px;
  color: white;
  position: relative;
  overflow: hidden;
}

header::before {
  content: '';
  position: absolute;
  top: -50%;
  left: -50%;
  width: 200%;
  height: 200%;
  background: radial-gradient(circle, rgba(255,255,255,0.1) 0%, transparent 60%);
  animation: pulse 4s ease-in-out infinite;
}

@keyframes pulse {
  0%, 100% { transform: scale(1); opacity: 0.5; }
  50% { transform: scale(1.1); opacity: 0.8; }
}

.header-content {
  display: flex;
  justify-content: space-between;
  align-items: center;
  position: relative;
  z-index: 1;
}

.header-title h1 {
  font-size: 2.5rem;
  color: white;
  margin-bottom: 0.5rem;
  text-shadow: 0 2px 4px rgba(0,0,0,0.1);
}

.header-title p {
  color: rgba(255, 255, 255, 0.9);
  font-size: 1.1rem;
}

.header-stats {
  display: flex;
  gap: 2rem;
}

.stat-item {
  text-align: center;
}

.stat-value {
  display: block;
  font-size: 2rem;
  font-weight: 700;
  color: white;
}

.stat-label {
  display: block;
  font-size: 0.9rem;
  color: rgba(255, 255, 255, 0.8);
  margin-top: 0.25rem;
}

.tabs {
  display: flex;
  gap: 0.5rem;
  margin-bottom: 1.5rem;
  background: white;
  padding: 0.5rem;
  border-radius: 12px;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.05);
}

.tab {
  flex: 1;
  padding: 0.75rem 1rem;
  border: none;
  border-radius: 8px;
  font-size: 0.95rem;
  cursor: pointer;
  transition: all 0.3s ease;
  background: transparent;
  color: #666;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 0.5rem;
  position: relative;
  overflow: hidden;
}

.tab::before {
  content: '';
  position: absolute;
  bottom: 0;
  left: 50%;
  width: 0;
  height: 3px;
  background: #3498db;
  transition: all 0.3s ease;
  transform: translateX(-50%);
}

.tab:hover {
  background: #f8f9fa;
  color: #2c3e50;
}

.tab:hover::before {
  width: 30%;
}

.tab.active {
  background: #f0f7ff;
  color: #3498db;
  font-weight: 600;
}

.tab.active::before {
  width: 100%;
  background: linear-gradient(90deg, #3498db, #2ecc71);
}

.badge {
  background: #e9ecef;
  padding: 0.15rem 0.5rem;
  border-radius: 12px;
  font-size: 0.75rem;
  font-weight: 600;
  transition: all 0.3s ease;
}

.tab.active .badge {
  background: linear-gradient(135deg, #3498db 0%, #2980b9 100%);
  color: white;
  box-shadow: 0 2px 4px rgba(52, 152, 219, 0.3);
}

.toolbar {
  display: flex;
  gap: 1rem;
  margin-bottom: 2rem;
  background: white;
  padding: 1rem;
  border-radius: 12px;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.05);
  align-items: center;
}

.search-bar {
  display: flex;
  gap: 1rem;
  flex: 1;
}

.search-bar input {
  flex: 1;
  padding: 0.75rem 1rem;
  border: 2px solid #e0e0e0;
  border-radius: 8px;
  font-size: 1rem;
  transition: all 0.3s ease;
  background: #f8f9fa;
  color: #2c3e50;
  caret-color: #3498db;
}

.search-bar input:focus {
  outline: none;
  border-color: #3498db;
  background: white;
  box-shadow: 0 0 0 3px rgba(52, 152, 219, 0.1);
}

.search-bar input::placeholder {
  color: #95a5a6;
}

.search-bar button {
  padding: 0.75rem 1.5rem;
  border: none;
  border-radius: 8px;
  font-size: 1rem;
  cursor: pointer;
  transition: background-color 0.3s;
  color: white;
  font-weight: 500;
}

.search-bar button:first-of-type {
  background-color: #3498db;
  color: white;
}

.search-bar button:first-of-type:hover {
  background-color: #2980b9;
}

.refresh-btn {
  background-color: #95a5a6;
  color: white;
}

.refresh-btn:hover {
  background-color: #7f8c8d;
}

.view-controls {
  display: flex;
  gap: 1rem;
  align-items: center;
}

.sort-controls {
  display: flex;
  gap: 0.5rem;
}

.sort-btn {
  padding: 0.5rem 1rem;
  border: 1px solid #e0e0e0;
  border-radius: 6px;
  background: white;
  color: #2c3e50;
  cursor: pointer;
  transition: all 0.3s ease;
  font-size: 0.875rem;
}

.sort-btn:hover {
  background: #f8f9fa;
  border-color: #3498db;
}

.sort-btn.active {
  background: #3498db;
  color: white;
  border-color: #3498db;
}

.view-toggle {
  display: flex;
  border: 1px solid #e0e0e0;
  border-radius: 6px;
  overflow: hidden;
}

.view-btn {
  padding: 0.5rem 1rem;
  border: none;
  background: white;
  color: #2c3e50;
  cursor: pointer;
  transition: all 0.3s ease;
  font-size: 0.875rem;
}

.view-btn:hover {
  background: #f8f9fa;
}

.view-btn.active {
  background: #3498db;
  color: white;
}

.view-btn:first-child {
  border-right: 1px solid #e0e0e0;
}

.loading {
  text-align: center;
  padding: 2rem;
  color: #7f8c8d;
}

.loading-spinner {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 1rem;
}

.spinner {
  width: 40px;
  height: 40px;
  border: 4px solid #e9ecef;
  border-top: 4px solid #3498db;
  border-radius: 50%;
  animation: spin 1s linear infinite;
}

@keyframes spin {
  0% { transform: rotate(0deg); }
  100% { transform: rotate(360deg); }
}

.loading-text {
  color: #7f8c8d;
  font-size: 1rem;
}

.progress-container {
  background: white;
  border-radius: 12px;
  padding: 1.5rem;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
  margin-bottom: 1rem;
}

.progress-info {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 0.75rem;
}

.progress-name {
  font-weight: 600;
  color: #2c3e50;
  font-size: 1.1rem;
}

.progress-step {
  color: #7f8c8d;
  font-size: 0.9rem;
}

.progress-bar {
  height: 8px;
  background: #e9ecef;
  border-radius: 4px;
  overflow: hidden;
  margin-bottom: 0.75rem;
}

.progress-fill {
  height: 100%;
  background: linear-gradient(90deg, #3498db, #2ecc71);
  border-radius: 4px;
  transition: width 0.3s ease;
}

.progress-message {
  color: #666;
  font-size: 0.9rem;
}

.agents-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(350px, 1fr));
  gap: 1.5rem;
}

.agents-table-container {
  background: white;
  border-radius: 12px;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.05);
  overflow: hidden;
}

.agents-table {
  width: 100%;
  border-collapse: collapse;
}

.agents-table th,
.agents-table td {
  padding: 1rem;
  text-align: left;
  border-bottom: 1px solid #e9ecef;
}

.agents-table th {
  background: #f8f9fa;
  font-weight: 600;
  color: #2c3e50;
  position: sticky;
  top: 0;
  z-index: 1;
}

.agents-table th.sortable {
  cursor: pointer;
  user-select: none;
}

.agents-table th.sortable:hover {
  background: #e9ecef;
}

.agents-table tbody tr:hover {
  background: #f8f9fa;
}

.agents-table tbody tr.selected {
  background: #e3f2fd;
}

.checkbox-col {
  width: 50px;
  text-align: center;
}

.name-col {
  font-weight: 600;
  color: #2c3e50;
}

.agent-name {
  font-size: 1rem;
}

.description-col {
  max-width: 300px;
  color: #666;
  font-size: 0.9rem;
}

.package-col code {
  background: #f8f9fa;
  padding: 0.25rem 0.5rem;
  border-radius: 4px;
  font-size: 0.85rem;
}

.manager-badge {
  background: #e3f2fd;
  color: #1976d2;
  padding: 0.25rem 0.5rem;
  border-radius: 4px;
  font-size: 0.85rem;
  font-weight: 500;
}

.source-col {
  color: #666;
  font-size: 0.9rem;
}

.status-badge {
  padding: 0.25rem 0.75rem;
  border-radius: 20px;
  font-size: 0.8rem;
  font-weight: 600;
}

.status-badge.installed {
  background: linear-gradient(135deg, #d4edda 0%, #c3e6cb 100%);
  color: #155724;
}

.status-badge.not-installed {
  background: linear-gradient(135deg, #f8d7da 0%, #f5c6cb 100%);
  color: #721c24;
}

.actions-col {
  white-space: nowrap;
}

.install-btn-sm,
.uninstall-btn-sm {
  padding: 0.4rem 0.8rem;
  border: none;
  border-radius: 4px;
  font-size: 0.8rem;
  cursor: pointer;
  transition: all 0.3s ease;
}

.install-btn-sm {
  background: #27ae60;
  color: white;
}

.install-btn-sm:hover:not(:disabled) {
  background: #219a52;
}

.uninstall-btn-sm {
  background: #e74c3c;
  color: white;
}

.uninstall-btn-sm:hover:not(:disabled) {
  background: #c0392b;
}

.agent-card {
  background: white;
  border-radius: 12px;
  padding: 1.5rem;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
  transition: all 0.3s ease;
  border-left: 4px solid transparent;
  position: relative;
  overflow: hidden;
  cursor: pointer;
}

.agent-card::before {
  content: '';
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  height: 3px;
  background: linear-gradient(90deg, transparent, rgba(255,255,255,0.8), transparent);
  opacity: 0;
  transition: opacity 0.3s ease;
}

.agent-card:hover {
  transform: translateY(-4px);
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.15);
}

.agent-card:hover::before {
  opacity: 1;
}

.agent-card.cli {
  border-left-color: #27ae60;
}

.agent-card.cli::after {
  content: '';
  position: absolute;
  top: 0;
  right: 0;
  width: 60px;
  height: 60px;
  background: linear-gradient(135deg, transparent 50%, rgba(39, 174, 96, 0.1) 50%);
  border-radius: 0 12px 0 0;
}

.agent-card.desktop {
  border-left-color: #9b59b6;
}

.agent-card.desktop::after {
  content: '';
  position: absolute;
  top: 0;
  right: 0;
  width: 60px;
  height: 60px;
  background: linear-gradient(135deg, transparent 50%, rgba(155, 89, 182, 0.1) 50%);
  border-radius: 0 12px 0 0;
}

.agent-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: 0.75rem;
}

.agent-header h3 {
  margin: 0;
  color: #2c3e50;
  font-size: 1.25rem;
}

.badges {
  display: flex;
  gap: 0.5rem;
  align-items: center;
}

.type-badge {
  padding: 0.25rem 0.75rem;
  border-radius: 20px;
  font-size: 0.7rem;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.type-badge.cli {
  background-color: #d4edda;
  color: #155724;
}

.type-badge.desktop {
  background-color: #e8daef;
  color: #6c3483;
}

.status {
  padding: 0.25rem 0.75rem;
  border-radius: 20px;
  font-size: 0.75rem;
  font-weight: 600;
  transition: all 0.3s ease;
}

.installed {
  background: linear-gradient(135deg, #d4edda 0%, #c3e6cb 100%);
  color: #155724;
  box-shadow: 0 2px 4px rgba(21, 87, 36, 0.1);
}

.not-installed {
  background: linear-gradient(135deg, #f8d7da 0%, #f5c6cb 100%);
  color: #721c24;
  box-shadow: 0 2px 4px rgba(114, 28, 36, 0.1);
}

.description {
  color: #666;
  margin-bottom: 1rem;
  line-height: 1.5;
}

.agent-meta {
  display: flex;
  gap: 0.5rem;
  margin-bottom: 1rem;
}

.package {
  background-color: #e9ecef;
  padding: 0.25rem 0.5rem;
  border-radius: 4px;
  font-size: 0.875rem;
  color: #495057;
}

.manager {
  background-color: #e3f2fd;
  padding: 0.25rem 0.5rem;
  border-radius: 4px;
  font-size: 0.875rem;
  color: #1976d2;
}

.install-source {
  background-color: #fff3cd;
  padding: 0.25rem 0.5rem;
  border-radius: 4px;
  font-size: 0.875rem;
  color: #856404;
}

.download-url {
  background-color: #d1ecf1;
  padding: 0.25rem 0.5rem;
  border-radius: 4px;
  font-size: 0.875rem;
  color: #0c5460;
}

.download-url a {
  color: #0c5460;
  text-decoration: none;
}

.download-url a:hover {
  text-decoration: underline;
}

.actions {
  display: flex;
  gap: 0.75rem;
}

.install-btn,
.uninstall-btn {
  flex: 1;
  padding: 0.5rem 1rem;
  border: none;
  border-radius: 6px;
  font-size: 0.875rem;
  cursor: pointer;
  transition: all 0.3s ease;
  position: relative;
  overflow: hidden;
}

.install-btn {
  background: linear-gradient(135deg, #27ae60 0%, #2ecc71 100%);
  color: white;
  box-shadow: 0 2px 8px rgba(39, 174, 96, 0.3);
}

.install-btn:hover:not(:disabled) {
  transform: translateY(-2px);
  box-shadow: 0 4px 12px rgba(39, 174, 96, 0.4);
}

.uninstall-btn {
  background: linear-gradient(135deg, #e74c3c 0%, #c0392b 100%);
  color: white;
  box-shadow: 0 2px 8px rgba(231, 76, 60, 0.3);
}

.uninstall-btn:hover:not(:disabled) {
  transform: translateY(-2px);
  box-shadow: 0 4px 12px rgba(231, 76, 60, 0.4);
}

.empty {
  text-align: center;
  padding: 3rem;
  color: #7f8c8d;
  font-size: 1.1rem;
}

.batch-actions {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 1.5rem;
  padding: 1rem 1.5rem;
  background: white;
  border-radius: 12px;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.05);
}

.select-all {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.select-all input {
  width: 18px;
  height: 18px;
}

.batch-buttons {
  display: flex;
  gap: 0.5rem;
}

.batch-install-btn,
.batch-uninstall-btn {
  padding: 0.5rem 1rem;
  border: none;
  border-radius: 6px;
  font-size: 0.875rem;
  cursor: pointer;
  transition: all 0.3s ease;
  position: relative;
  overflow: hidden;
}

.batch-install-btn {
  background: linear-gradient(135deg, #27ae60 0%, #2ecc71 100%);
  color: white;
  box-shadow: 0 2px 8px rgba(39, 174, 96, 0.3);
}

.batch-install-btn:hover:not(:disabled) {
  transform: translateY(-2px);
  box-shadow: 0 4px 12px rgba(39, 174, 96, 0.4);
}

.batch-uninstall-btn {
  background: linear-gradient(135deg, #e74c3c 0%, #c0392b 100%);
  color: white;
  box-shadow: 0 2px 8px rgba(231, 76, 60, 0.3);
}

.batch-uninstall-btn:hover:not(:disabled) {
  transform: translateY(-2px);
  box-shadow: 0 4px 12px rgba(231, 76, 60, 0.4);
}

.agent-title {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.agent-title input {
  width: 18px;
  height: 18px;
}

.message {
  padding: 1rem;
  border-radius: 8px;
  margin-bottom: 1rem;
  text-align: center;
}

.success {
  background-color: #d4edda;
  color: #155724;
}

.error {
  background-color: #f8d7da;
  color: #721c24;
}

button:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

/* Desktop optimizations */
@media (min-width: 1200px) {
  .agents-grid {
    grid-template-columns: repeat(auto-fill, minmax(380px, 1fr));
  }
}

@media (min-width: 1600px) {
  .agents-grid {
    grid-template-columns: repeat(4, 1fr);
  }
}

/* Tooltip styles */
.tooltip {
  position: relative;
}

.tooltip::after {
  content: attr(data-tooltip);
  position: absolute;
  bottom: 100%;
  left: 50%;
  transform: translateX(-50%);
  background: #2c3e50;
  color: white;
  padding: 0.5rem 1rem;
  border-radius: 6px;
  font-size: 0.8rem;
  white-space: nowrap;
  opacity: 0;
  visibility: hidden;
  transition: all 0.3s ease;
  z-index: 100;
}

.tooltip:hover::after {
  opacity: 1;
  visibility: visible;
}

/* Keyboard shortcuts hint */
.shortcut-hint {
  display: inline-block;
  background: #e9ecef;
  padding: 0.1rem 0.3rem;
  border-radius: 3px;
  font-size: 0.7rem;
  color: #666;
  margin-left: 0.5rem;
}

.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.3s ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}

.message-icon {
  margin-right: 0.5rem;
  font-weight: bold;
}

/* Modal Styles */
.modal-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
  backdrop-filter: blur(4px);
}

.modal-content {
  background: white;
  border-radius: 16px;
  width: 90%;
  max-width: 600px;
  max-height: 80vh;
  overflow-y: auto;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3);
  animation: modalSlideIn 0.3s ease;
}

@keyframes modalSlideIn {
  from {
    transform: translateY(-20px);
    opacity: 0;
  }
  to {
    transform: translateY(0);
    opacity: 1;
  }
}

.modal-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  padding: 1.5rem;
  border-bottom: 1px solid #e9ecef;
}

.modal-title {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  flex-wrap: wrap;
}

.modal-title h2 {
  margin: 0;
  color: #2c3e50;
  font-size: 1.5rem;
}

.modal-close {
  background: none;
  border: none;
  font-size: 1.5rem;
  cursor: pointer;
  color: #666;
  padding: 0.5rem;
  line-height: 1;
  border-radius: 4px;
  transition: background-color 0.2s;
}

.modal-close:hover {
  background: #f0f0f0;
}

.modal-body {
  padding: 1.5rem;
}

.modal-description {
  color: #666;
  line-height: 1.6;
  margin-bottom: 1.5rem;
}

.detail-grid {
  display: grid;
  gap: 1rem;
}

.detail-item {
  display: flex;
  align-items: center;
  gap: 1rem;
}

.detail-label {
  min-width: 100px;
  font-weight: 600;
  color: #2c3e50;
}

.detail-value {
  color: #666;
}

.detail-value.link {
  color: #3498db;
  text-decoration: none;
}

.detail-value.link:hover {
  text-decoration: underline;
}

.modal-footer {
  display: flex;
  justify-content: flex-end;
  gap: 0.75rem;
  padding: 1.5rem;
  border-top: 1px solid #e9ecef;
}

.modal-install-btn,
.modal-uninstall-btn,
.modal-cancel-btn {
  padding: 0.75rem 1.5rem;
  border: none;
  border-radius: 8px;
  font-size: 1rem;
  cursor: pointer;
  transition: all 0.3s ease;
}

.modal-install-btn {
  background: linear-gradient(135deg, #27ae60 0%, #2ecc71 100%);
  color: white;
  box-shadow: 0 2px 8px rgba(39, 174, 96, 0.3);
}

.modal-install-btn:hover:not(:disabled) {
  transform: translateY(-2px);
  box-shadow: 0 4px 12px rgba(39, 174, 96, 0.4);
}

.modal-uninstall-btn {
  background: linear-gradient(135deg, #e74c3c 0%, #c0392b 100%);
  color: white;
  box-shadow: 0 2px 8px rgba(231, 76, 60, 0.3);
}

.modal-uninstall-btn:hover:not(:disabled) {
  transform: translateY(-2px);
  box-shadow: 0 4px 12px rgba(231, 76, 60, 0.4);
}

.modal-cancel-btn {
  background: #e9ecef;
  color: #666;
}

.modal-cancel-btn:hover {
  background: #dee2e6;
}

/* Responsive Design */
@media (max-width: 1200px) {
  .agents-grid {
    grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
  }
}

@media (max-width: 992px) {
  .container {
    padding: 1rem;
  }

  .header-content {
    flex-direction: column;
    gap: 1rem;
    text-align: center;
  }

  .header-stats {
    justify-content: center;
  }

  .toolbar {
    flex-direction: column;
    gap: 1rem;
  }

  .search-bar {
    flex-direction: column;
  }

  .view-controls {
    flex-direction: column;
    gap: 0.75rem;
  }

  .sort-controls {
    flex-wrap: wrap;
    justify-content: center;
  }

  .batch-actions {
    flex-direction: column;
    gap: 1rem;
    text-align: center;
  }

  .batch-buttons {
    flex-wrap: wrap;
    justify-content: center;
  }

  .detail-item {
    flex-direction: column;
    align-items: flex-start;
    gap: 0.25rem;
  }

  .detail-label {
    min-width: auto;
  }
}

@media (max-width: 768px) {
  .agents-grid {
    grid-template-columns: 1fr;
  }

  .header-title h1 {
    font-size: 2rem;
  }

  .tabs {
    flex-wrap: wrap;
  }

  .tab {
    flex: 1 1 calc(50% - 0.5rem);
  }

  .agents-table-container {
    overflow-x: auto;
  }

  .agents-table {
    min-width: 800px;
  }

  .modal-content {
    width: 95%;
    max-height: 90vh;
  }

  .modal-header {
    flex-direction: column;
    gap: 1rem;
  }

  .modal-title {
    flex-wrap: wrap;
  }

  .modal-footer {
    flex-direction: column;
  }

  .modal-install-btn,
  .modal-uninstall-btn,
  .modal-cancel-btn {
    width: 100%;
  }
}

@media (max-width: 480px) {
  .container {
    padding: 0.75rem;
  }

  .header-title h1 {
    font-size: 1.5rem;
  }

  .header-stats {
    gap: 1rem;
  }

  .stat-value {
    font-size: 1.5rem;
  }

  .tab {
    flex: 1 1 100%;
    font-size: 0.875rem;
    padding: 0.5rem 0.75rem;
  }

  .search-bar button {
    width: 100%;
  }

  .sort-controls {
    gap: 0.25rem;
  }

  .sort-btn {
    padding: 0.4rem 0.75rem;
    font-size: 0.75rem;
  }

  .view-toggle {
    width: 100%;
  }

  .view-btn {
    flex: 1;
  }

  .agent-card {
    padding: 1rem;
  }

  .agent-header {
    flex-direction: column;
    gap: 0.5rem;
  }

  .badges {
    flex-wrap: wrap;
  }

  .batch-actions {
    padding: 0.75rem;
  }

  .batch-install-btn,
  .batch-uninstall-btn {
    width: 100%;
  }
}
</style>
