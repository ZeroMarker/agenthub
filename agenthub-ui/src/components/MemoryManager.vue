<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'

interface MemoryInfo {
  path: string
  title: string
  content: string
  scope: string
  memory_type: string
  tags: string[]
  updated_at: string
}

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

function showMessage(msg: string, type: 'success' | 'error') {
  message.value = msg
  messageType.value = type
  setTimeout(() => message.value = '', 3000)
}

onMounted(() => loadMemories('all'))
</script>

<template>
  <div class="memory-manager">
    <header class="page-header">
      <h1>Memory Manager</h1>
      <p>Manage your persistent knowledge base</p>
    </header>

    <div v-if="message" :class="['message', messageType]">{{ message }}</div>

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
        <div v-if="loading && memories.length === 0" class="loading">Loading...</div>
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
        <p v-if="memories.length === 0 && !loading" class="empty">No memories found</p>
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
  </div>
</template>

<style scoped>
.memory-manager { padding: 2rem; }
.page-header { margin-bottom: 2rem; }
.page-header h1 { font-size: 1.75rem; color: #2c3e50; margin-bottom: 0.5rem; }
.page-header p { color: #666; }
.message { padding: 0.75rem 1rem; border-radius: 8px; margin-bottom: 1rem; }
.message.success { background: #d4edda; color: #155724; }
.message.error { background: #f8d7da; color: #721c24; }
.toolbar { display: flex; gap: 1rem; align-items: center; margin-bottom: 1.5rem; flex-wrap: wrap; }
.scope-tabs { display: flex; gap: 0.25rem; background: white; padding: 0.25rem; border-radius: 8px; box-shadow: 0 2px 8px rgba(0,0,0,0.05); }
.tab { padding: 0.5rem 1rem; border: none; border-radius: 6px; background: transparent; cursor: pointer; font-size: 0.9rem; }
.tab.active { background: #3498db; color: white; }
.search-box { display: flex; gap: 0.5rem; flex: 1; }
.search-box input { flex: 1; padding: 0.5rem 1rem; border: 1px solid #e0e0e0; border-radius: 8px; }
.search-box button { padding: 0.5rem 1rem; background: #3498db; color: white; border: none; border-radius: 8px; cursor: pointer; }
.create-btn { padding: 0.5rem 1rem; background: #27ae60; color: white; border: none; border-radius: 8px; cursor: pointer; }
.create-form { background: white; padding: 1.5rem; border-radius: 12px; box-shadow: 0 2px 8px rgba(0,0,0,0.05); margin-bottom: 1.5rem; }
.create-form h3 { margin-bottom: 1rem; color: #2c3e50; }
.form-row { display: flex; gap: 0.75rem; margin-bottom: 0.75rem; }
.form-row input, .form-row select { flex: 1; padding: 0.6rem 1rem; border: 1px solid #e0e0e0; border-radius: 8px; }
.create-form textarea { width: 100%; padding: 0.6rem 1rem; border: 1px solid #e0e0e0; border-radius: 8px; resize: vertical; margin-bottom: 0.75rem; }
.create-form button { padding: 0.6rem 1.5rem; background: #27ae60; color: white; border: none; border-radius: 8px; cursor: pointer; }
.content-layout { display: flex; gap: 2rem; }
.memory-list { width: 320px; flex-shrink: 0; background: white; padding: 1.5rem; border-radius: 12px; box-shadow: 0 2px 8px rgba(0,0,0,0.05); max-height: calc(100vh - 300px); overflow-y: auto; }
.memory-list h3 { margin-bottom: 1rem; color: #2c3e50; }
.memory-item { padding: 0.75rem; border-radius: 8px; cursor: pointer; margin-bottom: 0.25rem; }
.memory-item:hover { background: #f8f9fa; }
.memory-item.active { background: #e3f2fd; }
.memory-info { display: flex; flex-direction: column; gap: 0.25rem; }
.memory-title { font-weight: 600; color: #2c3e50; display: flex; align-items: center; gap: 0.5rem; }
.scope-icon { font-size: 1rem; }
.memory-meta { display: flex; gap: 0.5rem; }
.type-badge { font-size: 0.75rem; color: #666; }
.memory-detail { flex: 1; background: white; padding: 2rem; border-radius: 12px; box-shadow: 0 2px 8px rgba(0,0,0,0.05); max-height: calc(100vh - 300px); overflow-y: auto; }
.detail-header { margin-bottom: 1.5rem; padding-bottom: 1rem; border-bottom: 1px solid #eee; }
.detail-header h2 { color: #2c3e50; margin-bottom: 0.75rem; display: flex; align-items: center; gap: 0.5rem; }
.detail-meta { display: flex; gap: 0.5rem; }
.badge { padding: 0.3rem 0.75rem; background: #f0f0f0; color: #666; border-radius: 20px; font-size: 0.8rem; }
.tags { display: flex; gap: 0.5rem; margin-bottom: 1.5rem; }
.tag { padding: 0.2rem 0.5rem; background: #e3f2fd; color: #1976d2; border-radius: 4px; font-size: 0.8rem; }
.content-preview h3 { margin-bottom: 0.75rem; color: #2c3e50; }
.content-preview pre { background: #f8f9fa; padding: 1rem; border-radius: 8px; overflow-x: auto; font-size: 0.9rem; line-height: 1.5; white-space: pre-wrap; }
.delete-btn { padding: 0.5rem 1rem; background: #e74c3c; color: white; border: none; border-radius: 6px; cursor: pointer; }
.loading, .empty { text-align: center; padding: 2rem; color: #999; }
</style>
