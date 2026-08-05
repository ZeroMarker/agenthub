<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import PageHeader from './common/PageHeader.vue'
import NotificationBar from './common/NotificationBar.vue'
import LoadingSpinner from './common/LoadingSpinner.vue'
import EmptyState from './common/EmptyState.vue'

interface PromptInfo {
  id: string
  name: string
  description: string
  template: string
  tags: string[]
  category: string | null
  version: number
}

const prompts = ref<PromptInfo[]>([])
const selectedPrompt = ref<PromptInfo | null>(null)
const showCreateForm = ref(false)
const loading = ref(false)
const message = ref('')
const messageType = ref<'success' | 'error'>('success')

// Create form
const newId = ref('')
const newName = ref('')
const newDescription = ref('')
const newTemplate = ref('')

// Render form
const renderVars = ref<Record<string, string>>({})
const renderResult = ref('')

async function loadPrompts() {
  loading.value = true
  try {
    prompts.value = await invoke<PromptInfo[]>('list_prompts')
  } catch (error) {
    showMessage(`Failed to load prompts: ${error}`, 'error')
  } finally {
    loading.value = false
  }
}

async function createPrompt() {
  if (!newId.value.trim() || !newName.value.trim()) return
  loading.value = true
  try {
    await invoke('create_prompt', {
      id: newId.value.trim(),
      name: newName.value.trim(),
      description: newDescription.value.trim(),
      template: newTemplate.value,
    })
    showCreateForm.value = false
    newId.value = ''
    newName.value = ''
    newDescription.value = ''
    newTemplate.value = ''
    await loadPrompts()
    showMessage('Prompt created', 'success')
  } catch (error) {
    showMessage(`Failed to create prompt: ${error}`, 'error')
  } finally {
    loading.value = false
  }
}

async function deletePrompt(id: string) {
  loading.value = true
  try {
    await invoke('delete_prompt', { id })
    selectedPrompt.value = null
    await loadPrompts()
    showMessage('Prompt deleted', 'success')
  } catch (error) {
    showMessage(`Failed to delete prompt: ${error}`, 'error')
  } finally {
    loading.value = false
  }
}

async function renderPrompt() {
  if (!selectedPrompt.value) return
  loading.value = true
  try {
    renderResult.value = await invoke('render_prompt', {
      id: selectedPrompt.value.id,
      vars: renderVars.value,
    })
  } catch (error) {
    showMessage(`Failed to render prompt: ${error}`, 'error')
  } finally {
    loading.value = false
  }
}

function selectPrompt(prompt: PromptInfo) {
  selectedPrompt.value = prompt
  renderResult.value = ''
  renderVars.value = {}
}

function showMessage(msg: string, type: 'success' | 'error') {
  message.value = msg
  messageType.value = type
  setTimeout(() => message.value = '', 3000)
}

onMounted(loadPrompts)
</script>

<template>
  <div class="prompt-manager">
    <PageHeader title="Prompt Manager" subtitle="Create and manage prompt templates" />

    <NotificationBar :message="message" :type="messageType" @close="message = ''" />

    <div class="actions">
      <button class="create-btn" @click="showCreateForm = !showCreateForm">
        {{ showCreateForm ? 'Cancel' : 'Create Prompt' }}
      </button>
    </div>

    <div v-if="showCreateForm" class="create-form">
      <h3>New Prompt</h3>
      <div class="form-grid">
        <input v-model="newId" placeholder="ID (e.g., code-review)" />
        <input v-model="newName" placeholder="Name" />
      </div>
      <input v-model="newDescription" placeholder="Description" class="full-width" />
      <textarea v-model="newTemplate" placeholder="Template content (use {{variable}} for variables)" rows="6" />
      <button @click="createPrompt" :disabled="loading || !newId.trim() || !newName.trim()">
        Create
      </button>
    </div>

    <div class="content-layout">
      <div class="prompt-list">
        <h3>Templates ({{ prompts.length }})</h3>
        <LoadingSpinner v-if="loading && prompts.length === 0" />
        <div v-else class="list-items">
          <div
            v-for="prompt in prompts"
            :key="prompt.id"
            :class="['prompt-item', { active: selectedPrompt?.id === prompt.id }]"
            @click="selectPrompt(prompt)"
          >
            <div class="prompt-info">
              <span class="prompt-name">{{ prompt.name }}</span>
              <span class="prompt-id">{{ prompt.id }}</span>
            </div>
            <span class="version">v{{ prompt.version }}</span>
          </div>
        </div>
        <EmptyState v-if="prompts.length === 0 && !loading" text="No prompts yet" />
      </div>

      <div class="prompt-detail" v-if="selectedPrompt">
        <h2>{{ selectedPrompt.name }}</h2>
        <p class="description">{{ selectedPrompt.description }}</p>

        <div class="tags" v-if="selectedPrompt.tags.length">
          <span v-for="tag in selectedPrompt.tags" :key="tag" class="tag">{{ tag }}</span>
        </div>

        <div class="template-preview">
          <h3>Template</h3>
          <pre>{{ selectedPrompt.template }}</pre>
        </div>

        <div class="render-section">
          <h3>Test Render</h3>
          <button @click="renderPrompt" :disabled="loading">Render</button>
          <pre v-if="renderResult" class="render-result">{{ renderResult }}</pre>
        </div>

        <div class="actions">
          <button class="delete-btn" @click="deletePrompt(selectedPrompt.id)" :disabled="loading">
            Delete
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.prompt-manager { padding: 2rem; }
.actions { margin-bottom: 1.5rem; }
.create-btn { padding: 0.6rem 1.5rem; background: #3498db; color: white; border: none; border-radius: 8px; cursor: pointer; }
.create-form { background: white; padding: 1.5rem; border-radius: 12px; box-shadow: 0 2px 8px rgba(0,0,0,0.05); margin-bottom: 1.5rem; }
.create-form h3 { margin-bottom: 1rem; color: #2c3e50; }
.form-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 0.75rem; margin-bottom: 0.75rem; }
.create-form input, .create-form textarea { padding: 0.6rem 1rem; border: 1px solid #e0e0e0; border-radius: 8px; font-size: 0.95rem; width: 100%; }
.create-form textarea { font-family: monospace; resize: vertical; margin-bottom: 0.75rem; }
.create-form button { padding: 0.6rem 1.5rem; background: #27ae60; color: white; border: none; border-radius: 8px; cursor: pointer; }
.content-layout { display: flex; gap: 2rem; }
.prompt-list { width: 300px; flex-shrink: 0; background: white; padding: 1.5rem; border-radius: 12px; box-shadow: 0 2px 8px rgba(0,0,0,0.05); }
.prompt-list h3 { margin-bottom: 1rem; color: #2c3e50; }
.prompt-item { display: flex; justify-content: space-between; align-items: center; padding: 0.75rem; border-radius: 8px; cursor: pointer; margin-bottom: 0.25rem; }
.prompt-item:hover { background: #f8f9fa; }
.prompt-item.active { background: #e3f2fd; }
.prompt-info { display: flex; flex-direction: column; }
.prompt-name { font-weight: 600; color: #2c3e50; }
.prompt-id { font-size: 0.8rem; color: #999; }
.version { font-size: 0.8rem; color: #666; }
.prompt-detail { flex: 1; background: white; padding: 2rem; border-radius: 12px; box-shadow: 0 2px 8px rgba(0,0,0,0.05); }
.prompt-detail h2 { color: #2c3e50; margin-bottom: 0.5rem; }
.description { color: #666; margin-bottom: 1rem; }
.tags { display: flex; gap: 0.5rem; margin-bottom: 1.5rem; }
.tag { padding: 0.2rem 0.5rem; background: #f0f0f0; color: #666; border-radius: 4px; font-size: 0.8rem; }
.template-preview, .render-section { margin-bottom: 1.5rem; }
.template-preview h3, .render-section h3 { margin-bottom: 0.75rem; color: #2c3e50; }
.template-preview pre, .render-result { background: #f8f9fa; padding: 1rem; border-radius: 8px; overflow-x: auto; font-size: 0.9rem; line-height: 1.5; }
.render-section button { padding: 0.5rem 1rem; background: #3498db; color: white; border: none; border-radius: 6px; cursor: pointer; margin-bottom: 1rem; }
.delete-btn { padding: 0.5rem 1rem; background: #e74c3c; color: white; border: none; border-radius: 6px; cursor: pointer; }
.list-items { max-height: calc(100vh - 350px); overflow-y: auto; }

/* Responsive */
@media (max-width: 900px) {
  .prompt-manager { padding: 1.25rem; }
  .form-grid { grid-template-columns: 1fr; }
  .content-layout { flex-direction: column; }
  .prompt-list { width: 100%; max-height: none; }
  .toolbar { flex-direction: column; }
  .search-box { width: 100%; }
}
@media (max-width: 600px) {
  .prompt-manager { padding: 1rem; }
}
</style>
