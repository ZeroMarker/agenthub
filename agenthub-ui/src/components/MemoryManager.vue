<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import PageHeader from './common/PageHeader.vue'
import NotificationBar from './common/NotificationBar.vue'
import LoadingSpinner from './common/LoadingSpinner.vue'
import EmptyState from './common/EmptyState.vue'

interface MemoryInfo {
  path: string
  title: string
  content: string
  scope: string
  memory_type: string
  tags: string[]
  updated_at: string
}

interface GraphSummary {
  node_count: number
  edge_count: number
  built_at: string
  top_entities: string[]
}

interface GraphNode {
  id: string
  label: string
  kind: string
  occurrences: number
  memories: string[]
}

interface GraphEdge {
  source: string
  target: string
  weight: number
}

const view = ref<'entries' | 'graph'>('entries')
const graphSummary = ref<GraphSummary | null>(null)
const graphNodes = ref<GraphNode[]>([])
const graphEdges = ref<GraphEdge[]>([])
const selectedEntity = ref('')
const entityNeighbors = ref<GraphEdge[]>([])
const graphLoading = ref(false)

const memories = ref<MemoryInfo[]>([])
const selectedMemory = ref<MemoryInfo | null>(null)
const loading = ref(false)
const message = ref('')
const messageType = ref<'success' | 'error'>('success')
const searchQuery = ref('')
const activeScope = ref<string>('all')

// Create form
const showCreateForm = ref(false)
const newTitle = ref('')
const newContent = ref('')
const newScope = ref('global')

async function loadMemories(scope?: string) {
  loading.value = true
  try {
    const scopeParam = scope === 'all' ? null : scope
    memories.value = await invoke<MemoryInfo[]>('list_memories', { scope: scopeParam || null })
  } catch (error) {
    showMessage(`Failed to load memories: ${error}`, 'error')
  } finally {
    loading.value = false
  }
}

async function searchMemories() {
  if (!searchQuery.value.trim()) {
    await loadMemories(activeScope.value)
    return
  }
  loading.value = true
  try {
    memories.value = await invoke<MemoryInfo[]>('search_memories', { query: searchQuery.value.trim() })
  } catch (error) {
    showMessage(`Failed to search memories: ${error}`, 'error')
  } finally {
    loading.value = false
  }
}

async function createMemory() {
  if (!newTitle.value.trim()) return
  loading.value = true
  try {
    await invoke('create_memory', {
      title: newTitle.value.trim(),
      content: newContent.value,
      scope: newScope.value,
    })
    showCreateForm.value = false
    newTitle.value = ''
    newContent.value = ''
    await loadMemories(activeScope.value)
    showMessage('Memory created', 'success')
  } catch (error) {
    showMessage(`Failed to create memory: ${error}`, 'error')
  } finally {
    loading.value = false
  }
}

async function deleteMemory(path: string) {
  loading.value = true
  try {
    await invoke('delete_memory', { path })
    selectedMemory.value = null
    await loadMemories(activeScope.value)
    showMessage('Memory deleted', 'success')
  } catch (error) {
    showMessage(`Failed to delete memory: ${error}`, 'error')
  } finally {
    loading.value = false
  }
}

function selectMemory(memory: MemoryInfo) {
  selectedMemory.value = memory
}

function setScope(scope: string) {
  activeScope.value = scope
  loadMemories(scope)
}

function getScopeIcon(scope: string): string {
  switch (scope) {
    case 'global': return '🌐'
    case 'project': return '📁'
    case 'session': return '💬'
    default: return '📝'
  }
}

function getTypeIcon(type: string): string {
  switch (type) {
    case 'pinned': return '📌'
    case 'learning': return '📚'
    case 'decision': return '✅'
    case 'reference': return '📖'
    case 'feedback': return '💬'
    default: return '📝'
  }
}

function formatDate(dateStr: string): string {
  return new Date(dateStr).toLocaleString()
}

async function loadGraph() {
  graphLoading.value = true
  try {
    const [summary, graph] = await Promise.all([
      invoke<GraphSummary>('build_memory_graph'),
      invoke<{ nodes: GraphNode[]; edges: GraphEdge[] }>('get_memory_graph'),
    ])
    graphSummary.value = summary
    graphNodes.value = graph.nodes
    graphEdges.value = graph.edges
    if (!selectedEntity.value && summary.top_entities.length) {
      selectedEntity.value = summary.top_entities[0]
      await loadNeighbors(summary.top_entities[0])
    }
  } catch (error) {
    showMessage(`Failed to load graph: ${error}`, 'error')
  } finally {
    graphLoading.value = false
  }
}

async function loadNeighbors(entity: string) {
  selectedEntity.value = entity
  try {
    entityNeighbors.value = await invoke<GraphEdge[]>('graph_neighbors', {
      entity,
      limit: 15,
    })
  } catch (error) {
    entityNeighbors.value = []
  }
}

function switchView(next: typeof view.value) {
  view.value = next
  if (next === 'graph') loadGraph()
}

function neighborName(edge: GraphEdge, self: string): string {
  return edge.source === self ? edge.target : edge.source
}

function showMessage(msg: string, type: 'success' | 'error') {
  message.value = msg
  messageType.value = type
  setTimeout(() => message.value = '', 3000)
}

onMounted(() => loadMemories('all'))
</script>

<template>
  <div class="memory-manager">
    <PageHeader title="Memory Manager" subtitle="Manage your persistent knowledge base" />

    <NotificationBar :message="message" :type="messageType" @close="message = ''" />

    <div class="m3-tabs" role="tablist" aria-label="Memory sections">
      <button
        v-for="t in [{ id: 'entries', label: 'Entries' }, { id: 'graph', label: 'Knowledge Graph' }]"
        :key="t.id"
        :class="['m3-tab', { active: view === t.id }]"
        role="tab"
        :aria-selected="view === t.id"
        @click="switchView(t.id as typeof view)"
      >
        {{ t.label }}
      </button>
    </div>

    <!-- ============ Entries ============ -->
    <template v-if="view === 'entries'">
    <div class="toolbar">
      <div class="scope-tabs">
        <button :class="['tab', { active: activeScope === 'all' }]" @click="setScope('all')">All</button>
        <button :class="['tab', { active: activeScope === 'global' }]" @click="setScope('global')">Global</button>
        <button :class="['tab', { active: activeScope === 'project' }]" @click="setScope('project')">Project</button>
        <button :class="['tab', { active: activeScope === 'session' }]" @click="setScope('session')">Session</button>
      </div>
      <div class="search-box">
        <input v-model="searchQuery" placeholder="Search memories..." @keyup.enter="searchMemories" />
        <button @click="searchMemories">Search</button>
      </div>
      <button class="create-btn" @click="showCreateForm = !showCreateForm">
        {{ showCreateForm ? 'Cancel' : 'Create' }}
      </button>
    </div>

    <div v-if="showCreateForm" class="create-form">
      <h3>New Memory</h3>
      <div class="form-row">
        <input v-model="newTitle" placeholder="Title" />
        <select v-model="newScope">
          <option value="global">Global</option>
          <option value="project">Project</option>
          <option value="session">Session</option>
        </select>
      </div>
      <textarea v-model="newContent" placeholder="Content" rows="4" />
      <button @click="createMemory" :disabled="loading || !newTitle.trim()">Save</button>
    </div>

    <div class="content-layout">
      <div class="memory-list">
        <h3>Entries ({{ memories.length }})</h3>
        <LoadingSpinner v-if="loading && memories.length === 0" />
        <div v-else class="list-items">
          <div
            v-for="memory in memories"
            :key="memory.path"
            :class="['memory-item', { active: selectedMemory?.path === memory.path }]"
            @click="selectMemory(memory)"
          >
            <div class="memory-info">
              <span class="memory-title">
                <span class="scope-icon">{{ getScopeIcon(memory.scope) }}</span>
                {{ memory.title }}
              </span>
              <span class="memory-meta">
                <span class="type-badge">{{ getTypeIcon(memory.memory_type) }} {{ memory.memory_type }}</span>
              </span>
            </div>
          </div>
        </div>
        <EmptyState v-if="memories.length === 0 && !loading" text="No memories found" />
      </div>

      <div class="memory-detail" v-if="selectedMemory">
        <div class="detail-header">
          <h2>
            <span class="scope-icon">{{ getScopeIcon(selectedMemory.scope) }}</span>
            {{ selectedMemory.title }}
          </h2>
          <div class="detail-meta">
            <span class="badge">{{ selectedMemory.scope }}</span>
            <span class="badge">{{ selectedMemory.memory_type }}</span>
            <span class="badge">{{ formatDate(selectedMemory.updated_at) }}</span>
          </div>
        </div>

        <div v-if="selectedMemory.tags.length" class="tags">
          <span v-for="tag in selectedMemory.tags" :key="tag" class="tag">{{ tag }}</span>
        </div>

        <div class="content-preview">
          <h3>Content</h3>
          <pre>{{ selectedMemory.content }}</pre>
        </div>

        <div class="actions">
          <button class="delete-btn" @click="deleteMemory(selectedMemory.path)" :disabled="loading">
            Delete
          </button>
        </div>
      </div>
    </div>
    </template>

    <!-- ============ Knowledge Graph ============ -->
    <template v-else>
      <LoadingSpinner v-if="graphLoading && !graphSummary" />
      <template v-else-if="graphSummary">
        <div class="stat-chips">
          <span class="stat-chip">{{ graphSummary.node_count }} entities</span>
          <span class="stat-chip">{{ graphSummary.edge_count }} relations</span>
          <span class="stat-chip">built {{ new Date(graphSummary.built_at).toLocaleString() }}</span>
        </div>

        <div class="graph-layout">
          <div class="graph-entities">
            <h3>Entities ({{ graphNodes.length }})</h3>
            <div class="entity-list">
              <button
                v-for="node in graphNodes"
                :key="node.id"
                :class="['entity-chip', { active: selectedEntity === node.id }]"
                @click="loadNeighbors(node.id)"
              >
                {{ node.label }}
                <span class="entity-count">{{ node.occurrences }}</span>
              </button>
            </div>
          </div>

          <div class="graph-detail">
            <h3>Relations of "{{ selectedEntity }}"</h3>
            <EmptyState v-if="entityNeighbors.length === 0" text="No relations found for this entity." />
            <div v-else class="neighbor-list">
              <div v-for="edge in entityNeighbors" :key="`${edge.source}-${edge.target}`" class="neighbor-row">
                <span class="neighbor-name">{{ neighborName(edge, selectedEntity) }}</span>
                <span class="neighbor-weight">×{{ edge.weight }}</span>
              </div>
            </div>
          </div>
        </div>
      </template>
    </template>
  </div>
</template>

<style scoped>
.memory-manager { padding: 2rem; }
.toolbar { display: flex; gap: 1rem; align-items: center; margin-bottom: 1.5rem; flex-wrap: wrap; }
.scope-tabs { display: flex; gap: 0.25rem; background: var(--md-sys-color-surface); padding: 0.25rem; border-radius: var(--md-sys-shape-sm); box-shadow: var(--md-sys-elevation-1); }
.tab { padding: 0.5rem 1rem; border: none; border-radius: var(--md-sys-shape-xs); background: transparent; cursor: pointer; font-size: 0.9rem; }
.tab.active { background: var(--md-sys-color-primary); color: var(--md-sys-color-on-primary); }
.search-box { display: flex; gap: 0.5rem; flex: 1; }
.search-box input { flex: 1; padding: 0.5rem 1rem; border: 1px solid var(--md-sys-color-outline-variant); border-radius: var(--md-sys-shape-sm); }
.search-box button { padding: 0.5rem 1rem; background: var(--md-sys-color-primary); color: var(--md-sys-color-on-primary); border: none; border-radius: var(--md-sys-shape-sm); cursor: pointer; }
.create-btn { padding: 0.5rem 1rem; background: var(--md-sys-color-primary); color: var(--md-sys-color-on-primary); border: none; border-radius: var(--md-sys-shape-sm); cursor: pointer; }
.create-form { background: var(--md-sys-color-surface); padding: 1.5rem; border-radius: var(--md-sys-shape-md); box-shadow: var(--md-sys-elevation-1); margin-bottom: 1.5rem; }
.create-form h3 { margin-bottom: 1rem; color: var(--md-sys-color-on-surface); }
.form-row { display: flex; gap: 0.75rem; margin-bottom: 0.75rem; }
.form-row input, .form-row select { flex: 1; padding: 0.6rem 1rem; border: 1px solid var(--md-sys-color-outline-variant); border-radius: var(--md-sys-shape-sm); }
.create-form textarea { width: 100%; padding: 0.6rem 1rem; border: 1px solid var(--md-sys-color-outline-variant); border-radius: var(--md-sys-shape-sm); resize: vertical; margin-bottom: 0.75rem; }
.create-form button { padding: 0.6rem 1.5rem; background: var(--md-sys-color-primary); color: var(--md-sys-color-on-primary); border: none; border-radius: var(--md-sys-shape-sm); cursor: pointer; }
.content-layout { display: flex; gap: 2rem; }
.memory-list { width: 320px; flex-shrink: 0; background: var(--md-sys-color-surface); padding: 1.5rem; border-radius: var(--md-sys-shape-xl); box-shadow: var(--md-sys-elevation-1); max-height: calc(100vh - 300px); overflow-y: auto; }
.memory-list h3 { margin-bottom: 1rem; color: var(--md-sys-color-on-surface); }
.memory-item { padding: 0.75rem; border-radius: var(--md-sys-shape-sm); cursor: pointer; margin-bottom: 0.25rem; }
.memory-item:hover { background: var(--md-sys-color-surface-variant); }
.memory-item.active { background: var(--md-sys-color-primary-container); }
.memory-info { display: flex; flex-direction: column; gap: 0.25rem; }
.memory-title { font-weight: 600; color: var(--md-sys-color-on-surface); display: flex; align-items: center; gap: 0.5rem; }
.scope-icon { font-size: 1rem; }
.memory-meta { display: flex; gap: 0.5rem; }
.type-badge { font-size: 0.75rem; color: var(--md-sys-color-on-surface-variant); }
.memory-detail { flex: 1; background: var(--md-sys-color-surface); padding: 2rem; border-radius: var(--md-sys-shape-md); box-shadow: var(--md-sys-elevation-1); max-height: calc(100vh - 300px); overflow-y: auto; }
.detail-header { margin-bottom: 1.5rem; padding-bottom: 1rem; border-bottom: 1px solid var(--md-sys-color-outline-variant); }
.detail-header h2 { color: var(--md-sys-color-on-surface); margin-bottom: 0.75rem; display: flex; align-items: center; gap: 0.5rem; }
.detail-meta { display: flex; gap: 0.5rem; }
.badge { padding: 0.3rem 0.75rem; background: var(--md-sys-color-surface-variant); color: var(--md-sys-color-on-surface-variant); border-radius: var(--md-sys-shape-xl); font-size: 0.8rem; }
.tags { display: flex; gap: 0.5rem; margin-bottom: 1.5rem; }
.tag { padding: 0.2rem 0.5rem; background: var(--md-sys-color-primary-container); color: var(--md-sys-color-primary); border-radius: var(--md-sys-shape-xs); font-size: 0.8rem; }
.content-preview h3 { margin-bottom: 0.75rem; color: var(--md-sys-color-on-surface); }
.content-preview pre { background: var(--md-sys-color-surface-variant); padding: 1rem; border-radius: var(--md-sys-shape-sm); overflow-x: auto; font-size: 0.9rem; line-height: 1.5; white-space: pre-wrap; }
.delete-btn { padding: 0.5rem 1rem; background: var(--md-sys-color-error); color: var(--md-sys-color-on-error); border: none; border-radius: var(--md-sys-shape-xs); cursor: pointer; }

/* Graph tab */
.stat-chips { display: flex; gap: 0.5rem; flex-wrap: wrap; margin-bottom: 1rem; }
.stat-chip { padding: 0.3rem 0.8rem; background: var(--md-sys-color-secondary-container); color: var(--md-sys-color-on-secondary-container); border-radius: var(--md-sys-shape-full); font-size: 0.8rem; }
.graph-layout { display: flex; gap: 2rem; }
.graph-entities { width: 340px; flex-shrink: 0; background: var(--md-sys-color-surface); padding: 1.5rem; border-radius: var(--md-sys-shape-md); box-shadow: var(--md-sys-elevation-1); }
.graph-entities h3, .graph-detail h3 { margin-bottom: 1rem; color: var(--md-sys-color-on-surface); }
.entity-list { display: flex; flex-wrap: wrap; gap: 0.5rem; max-height: calc(100vh - 380px); overflow-y: auto; }
.entity-chip { display: inline-flex; align-items: center; gap: 0.4rem; padding: 0.35rem 0.8rem; border: 1px solid var(--md-sys-color-outline-variant); border-radius: var(--md-sys-shape-full); background: transparent; color: var(--md-sys-color-on-surface); font-size: 0.85rem; cursor: pointer; }
.entity-chip.active { background: var(--md-sys-color-primary-container); border-color: transparent; color: var(--md-sys-color-on-primary-container); }
.entity-count { font-size: 0.7rem; background: var(--md-sys-color-surface-variant); border-radius: var(--md-sys-shape-full); padding: 0.05rem 0.4rem; }
.graph-detail { flex: 1; background: var(--md-sys-color-surface); padding: 1.5rem; border-radius: var(--md-sys-shape-md); box-shadow: var(--md-sys-elevation-1); }
.neighbor-list { display: flex; flex-direction: column; gap: 0.4rem; }
.neighbor-row { display: flex; justify-content: space-between; align-items: center; padding: 0.6rem 0.9rem; background: var(--md-sys-color-surface-variant); border-radius: var(--md-sys-shape-sm); }
.neighbor-name { font-weight: 600; color: var(--md-sys-color-on-surface); }
.neighbor-weight { color: var(--md-sys-color-on-surface-variant); font-size: 0.85rem; }
@media (max-width: 900px) {
  .memory-manager { padding: 1.25rem; }
  .toolbar { flex-direction: column; align-items: stretch; }
  .scope-tabs { overflow-x: auto; }
  .search-box { width: 100%; }
  .content-layout { flex-direction: column; }
  .memory-list { width: 100%; max-height: none; }
  .form-row { flex-direction: column; }
  .graph-layout { flex-direction: column; }
  .graph-entities { width: 100%; }
}
@media (max-width: 600px) {
  .memory-manager { padding: 1rem; }
}
</style>
