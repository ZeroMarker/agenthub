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

interface CommunityPromptInfo {
  id: string
  name: string
  description: string
  template: string
  tags: string[]
  category: string | null
  version: number
  publisher: string
  published_at: string
  source: string | null
}

const view = ref<'templates' | 'community'>('templates')
const communityPrompts = ref<CommunityPromptInfo[]>([])
const selectedCommunity = ref<CommunityPromptInfo | null>(null)
const publisherName = ref('local')
const installNewId = ref('')
const communityLoading = ref(false)

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

async function loadCommunity() {
  communityLoading.value = true
  try {
    communityPrompts.value = await invoke<CommunityPromptInfo[]>('list_community_prompts')
  } catch (error) {
    showMessage(`Failed to load community prompts: ${error}`, 'error')
  } finally {
    communityLoading.value = false
  }
}

async function publishSelected() {
  if (!selectedPrompt.value) return
  communityLoading.value = true
  try {
    await invoke('publish_prompt', {
      id: selectedPrompt.value.id,
      publisher: publisherName.value.trim() || 'local',
      force: false,
    })
    await loadCommunity()
    showMessage(`Published '${selectedPrompt.value.id}' to community`, 'success')
  } catch (error) {
    showMessage(`Failed to publish: ${error}`, 'error')
  } finally {
    communityLoading.value = false
  }
}

async function installCommunity(prompt: CommunityPromptInfo) {
  communityLoading.value = true
  try {
    await invoke('install_community_prompt', {
      id: prompt.id,
      newId: installNewId.value.trim() || null,
      force: false,
    })
    installNewId.value = ''
    await loadCommunity()
    await loadPrompts()
    showMessage(`Installed '${prompt.id}' as a local template`, 'success')
  } catch (error) {
    showMessage(`Failed to install: ${error}`, 'error')
  } finally {
    communityLoading.value = false
  }
}

async function deleteCommunity(prompt: CommunityPromptInfo) {
  communityLoading.value = true
  try {
    await invoke('delete_community_prompt', { id: prompt.id })
    selectedCommunity.value = null
    await loadCommunity()
    showMessage('Community prompt deleted', 'success')
  } catch (error) {
    showMessage(`Failed to delete: ${error}`, 'error')
  } finally {
    communityLoading.value = false
  }
}

function switchView(next: typeof view.value) {
  view.value = next
  if (next === 'community') loadCommunity()
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
    <PageHeader title="Prompt Manager" subtitle="Create, share and manage prompt templates" />

    <NotificationBar :message="message" :type="messageType" @close="message = ''" />

    <div class="m3-tabs" role="tablist" aria-label="Prompt sections">
      <button
        v-for="t in [{ id: 'templates', label: 'Templates' }, { id: 'community', label: 'Community' }]"
        :key="t.id"
        :class="['m3-tab', { active: view === t.id }]"
        role="tab"
        :aria-selected="view === t.id"
        @click="switchView(t.id as typeof view)"
      >
        {{ t.label }}
      </button>
    </div>

    <!-- ============ Templates ============ -->
    <template v-if="view === 'templates'">
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

        <div class="actions publish-box">
          <h3>Publish to community</h3>
          <div class="publish-row">
            <input v-model="publisherName" placeholder="Publisher" />
            <button class="create-btn" @click="publishSelected" :disabled="communityLoading">
              Publish
            </button>
          </div>
        </div>
      </div>
    </div>
    </template>

    <!-- ============ Community ============ -->
    <template v-else>
      <div class="actions">
        <span class="hint">Community prompts are local snapshots you can publish and install — share the
        <code>prompts/community</code> directory (e.g. via git) to collaborate offline.</span>
      </div>
      <div class="content-layout">
        <div class="prompt-list">
          <h3>Community ({{ communityPrompts.length }})</h3>
          <LoadingSpinner v-if="communityLoading && communityPrompts.length === 0" />
          <div v-else class="list-items">
            <div
              v-for="prompt in communityPrompts"
              :key="prompt.id"
              :class="['prompt-item', { active: selectedCommunity?.id === prompt.id }]"
              @click="selectedCommunity = prompt"
            >
              <div class="prompt-info">
                <span class="prompt-name">{{ prompt.name }}</span>
                <span class="prompt-id">{{ prompt.id }} · by {{ prompt.publisher }}</span>
              </div>
              <span class="version">v{{ prompt.version }}</span>
            </div>
          </div>
          <EmptyState v-if="communityPrompts.length === 0 && !communityLoading" text="No community prompts yet" />
        </div>

        <div class="prompt-detail" v-if="selectedCommunity">
          <h2>{{ selectedCommunity.name }}</h2>
          <p class="description">{{ selectedCommunity.description }}</p>
          <p class="description">
            Published {{ new Date(selectedCommunity.published_at).toLocaleDateString() }} by
            {{ selectedCommunity.publisher }} · source {{ selectedCommunity.source ?? '-' }}
          </p>
          <div class="tags" v-if="selectedCommunity.tags.length">
            <span v-for="tag in selectedCommunity.tags" :key="tag" class="tag">{{ tag }}</span>
          </div>
          <div class="template-preview">
            <h3>Template</h3>
            <pre>{{ selectedCommunity.template }}</pre>
          </div>
          <div class="actions install-box">
            <input v-model="installNewId" placeholder="Install as id (optional)" />
            <button class="create-btn" @click="installCommunity(selectedCommunity)" :disabled="communityLoading">
              Install as template
            </button>
            <button class="delete-btn" @click="deleteCommunity(selectedCommunity)" :disabled="communityLoading">
              Delete
            </button>
          </div>
        </div>
      </div>
    </template>
  </div>
</template>

<style scoped>
.prompt-manager { padding: 2rem; }
.actions { margin-bottom: 1.5rem; }
.create-btn { padding: 0.6rem 1.5rem; background: var(--md-sys-color-primary); color: var(--md-sys-color-on-primary); border: none; border-radius: var(--md-sys-shape-sm); cursor: pointer; }
.create-form { background: var(--md-sys-color-surface); padding: 1.5rem; border-radius: var(--md-sys-shape-md); box-shadow: var(--md-sys-elevation-1); margin-bottom: 1.5rem; }
.create-form h3 { margin-bottom: 1rem; color: var(--md-sys-color-on-surface); }
.form-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 0.75rem; margin-bottom: 0.75rem; }
.create-form input, .create-form textarea { padding: 0.6rem 1rem; border: 1px solid var(--md-sys-color-outline-variant); border-radius: var(--md-sys-shape-sm); font-size: 0.95rem; width: 100%; }
.create-form textarea { font-family: monospace; resize: vertical; margin-bottom: 0.75rem; }
.create-form button { padding: 0.6rem 1.5rem; background: var(--md-sys-color-primary); color: var(--md-sys-color-on-primary); border: none; border-radius: var(--md-sys-shape-sm); cursor: pointer; }
.content-layout { display: flex; gap: 2rem; }
.prompt-list { width: 300px; flex-shrink: 0; background: var(--md-sys-color-surface); padding: 1.5rem; border-radius: var(--md-sys-shape-md); box-shadow: var(--md-sys-elevation-1); }
.prompt-list h3 { margin-bottom: 1rem; color: var(--md-sys-color-on-surface); }
.prompt-item { display: flex; justify-content: space-between; align-items: center; padding: 0.75rem; border-radius: var(--md-sys-shape-sm); cursor: pointer; margin-bottom: 0.25rem; }
.prompt-item:hover { background: var(--md-sys-color-surface-variant); }
.prompt-item.active { background: var(--md-sys-color-primary-container); }
.prompt-info { display: flex; flex-direction: column; }
.prompt-name { font-weight: 600; color: var(--md-sys-color-on-surface); }
.prompt-id { font-size: 0.8rem; color: var(--md-sys-color-on-surface-variant); }
.version { font-size: 0.8rem; color: var(--md-sys-color-on-surface-variant); }
.prompt-detail { flex: 1; background: var(--md-sys-color-surface); padding: 2rem; border-radius: var(--md-sys-shape-md); box-shadow: var(--md-sys-elevation-1); }
.prompt-detail h2 { color: var(--md-sys-color-on-surface); margin-bottom: 0.5rem; }
.description { color: var(--md-sys-color-on-surface-variant); margin-bottom: 1rem; }
.tags { display: flex; gap: 0.5rem; margin-bottom: 1.5rem; }
.tag { padding: 0.2rem 0.5rem; background: var(--md-sys-color-surface-variant); color: var(--md-sys-color-on-surface-variant); border-radius: var(--md-sys-shape-xs); font-size: 0.8rem; }
.template-preview, .render-section { margin-bottom: 1.5rem; }
.template-preview h3, .render-section h3 { margin-bottom: 0.75rem; color: var(--md-sys-color-on-surface); }
.template-preview pre, .render-result { background: var(--md-sys-color-surface-variant); padding: 1rem; border-radius: var(--md-sys-shape-sm); overflow-x: auto; font-size: 0.9rem; line-height: 1.5; }
.render-section button { padding: 0.5rem 1rem; background: var(--md-sys-color-primary); color: var(--md-sys-color-on-primary); border: none; border-radius: var(--md-sys-shape-xs); cursor: pointer; margin-bottom: 1rem; }
.delete-btn { padding: 0.5rem 1rem; background: var(--md-sys-color-error); color: var(--md-sys-color-on-primary); border: none; border-radius: var(--md-sys-shape-xs); cursor: pointer; }
.list-items { max-height: calc(100vh - 350px); overflow-y: auto; }

/* Community tab */
.m3-tabs { display: flex; gap: 0.25rem; background: var(--md-sys-color-surface-variant); border-radius: var(--md-sys-shape-sm); padding: 0.25rem; margin-bottom: 1.5rem; width: fit-content; }
.m3-tab { padding: 0.5rem 1.25rem; border: none; background: transparent; border-radius: var(--md-sys-shape-xs); color: var(--md-sys-color-on-surface-variant); font: var(--md-sys-typescale-label-large); cursor: pointer; }
.m3-tab.active { background: var(--md-sys-color-secondary-container); color: var(--md-sys-color-on-secondary-container); }
.hint { color: var(--md-sys-color-on-surface-variant); font-size: 0.9rem; }
.hint code { background: var(--md-sys-color-surface-variant); padding: 0.1rem 0.35rem; border-radius: var(--md-sys-shape-xs); }
.publish-box, .install-box { display: flex; align-items: center; gap: 0.75rem; border-top: 1px solid var(--md-sys-color-outline-variant); padding-top: 1rem; margin-top: 1rem; }
.publish-box h3 { margin: 0; font-size: 0.95rem; color: var(--md-sys-color-on-surface); }
.publish-row { display: flex; gap: 0.6rem; align-items: center; flex: 1; }
.publish-box input, .install-box input { flex: 1; padding: 0.5rem 0.9rem; border: 1px solid var(--md-sys-color-outline-variant); border-radius: var(--md-sys-shape-sm); background: var(--md-sys-color-surface); color: var(--md-sys-color-on-surface); }

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
