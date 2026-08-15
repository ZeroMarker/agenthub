<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import PageHeader from './common/PageHeader.vue'
import NotificationBar from './common/NotificationBar.vue'
import LoadingSpinner from './common/LoadingSpinner.vue'
import EmptyState from './common/EmptyState.vue'

interface SessionInfo {
  id: string
  title: string
  agent: string
  status: string
  started_at: string
  ended_at: string | null
  message_count: number
  tags: string[]
}

const sessions = ref<SessionInfo[]>([])
const selectedSession = ref<SessionInfo | null>(null)
const loading = ref(false)
const message = ref('')
const messageType = ref<'success' | 'error'>('success')

// Create form
const newTitle = ref('')
const newAgent = ref('')

async function loadSessions() {
  loading.value = true
  try {
    sessions.value = await invoke<SessionInfo[]>('list_sessions')
  } catch (error) {
    showMessage(`Failed to load sessions: ${error}`, 'error')
  } finally {
    loading.value = false
  }
}

async function createSession() {
  if (!newTitle.value.trim() || !newAgent.value.trim()) return
  loading.value = true
  try {
    await invoke('create_session', {
      title: newTitle.value.trim(),
      agent: newAgent.value.trim(),
    })
    newTitle.value = ''
    newAgent.value = ''
    await loadSessions()
    showMessage('Session created', 'success')
  } catch (error) {
    showMessage(`Failed to create session: ${error}`, 'error')
  } finally {
    loading.value = false
  }
}

async function deleteSession(id: string, title?: string) {
  if (!confirm(`Delete session${title ? ` '${title}'` : ''}? This cannot be undone.`)) return
  loading.value = true
  try {
    await invoke('delete_session', { id })
    selectedSession.value = null
    await loadSessions()
    showMessage('Session deleted', 'success')
  } catch (error) {
    showMessage(`Failed to delete session: ${error}`, 'error')
  } finally {
    loading.value = false
  }
}

function selectSession(session: SessionInfo) {
  selectedSession.value = session
}

function getStatusClass(status: string): string {
  switch (status) {
    case 'active': return 'active'
    case 'completed': return 'completed'
    case 'failed': return 'failed'
    case 'paused': return 'paused'
    default: return ''
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

onMounted(loadSessions)
</script>

<template>
  <div class="session-manager">
    <PageHeader title="Session Manager" subtitle="Track and manage your agent sessions" />

    <NotificationBar :message="message" :type="messageType" @close="message = ''" />

    <div class="create-section">
      <h3>Create Session</h3>
      <div class="create-form">
        <input v-model="newTitle" placeholder="Session title" />
        <input v-model="newAgent" placeholder="Agent name" @keyup.enter="createSession" />
        <button @click="createSession" :disabled="loading || !newTitle.trim() || !newAgent.trim()">
          Create
        </button>
      </div>
    </div>

    <div class="content-layout">
      <div class="session-list">
        <h3>Sessions ({{ sessions.length }})</h3>
        <LoadingSpinner v-if="loading && sessions.length === 0" />
        <div v-else class="list-items">
          <div
            v-for="session in sessions"
            :key="session.id"
            :class="['session-item', { active: selectedSession?.id === session.id }]"
            role="button"
            tabindex="0"
            :aria-label="`View session ${session.title}`"
            @click="selectSession(session)"
            @keydown.enter.prevent="selectSession(session)"
            @keydown.space.prevent="selectSession(session)"
          >
            <div class="session-info">
              <span class="session-title">{{ session.title }}</span>
              <span class="session-agent">{{ session.agent }}</span>
            </div>
            <span :class="['status-badge', getStatusClass(session.status)]">
              {{ session.status }}
            </span>
          </div>
        </div>
        <EmptyState v-if="sessions.length === 0 && !loading" text="No sessions yet" />
      </div>

      <div class="session-detail" v-if="selectedSession">
        <h2>{{ selectedSession.title }}</h2>
        <div class="detail-meta">
          <span class="badge">Agent: {{ selectedSession.agent }}</span>
          <span :class="['badge', 'status-' + selectedSession.status]">{{ selectedSession.status }}</span>
          <span class="badge">Messages: {{ selectedSession.message_count }}</span>
        </div>

        <div class="timeline">
          <div class="timeline-item">
            <span class="label">Started</span>
            <span class="value">{{ formatDate(selectedSession.started_at) }}</span>
          </div>
          <div v-if="selectedSession.ended_at" class="timeline-item">
            <span class="label">Ended</span>
            <span class="value">{{ formatDate(selectedSession.ended_at) }}</span>
          </div>
        </div>

        <div v-if="selectedSession.tags.length" class="tags">
          <span v-for="tag in selectedSession.tags" :key="tag" class="tag">{{ tag }}</span>
        </div>

        <div class="actions">
          <button class="delete-btn" @click="deleteSession(selectedSession.id, selectedSession.title)" :disabled="loading">
            Delete Session
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.session-manager { padding: 2rem; }
.create-section { background: var(--md-sys-color-surface); padding: 1.5rem; border-radius: var(--md-sys-shape-md); box-shadow: var(--md-sys-elevation-1); margin-bottom: 1.5rem; }
.create-section h3 { margin-bottom: 1rem; color: var(--md-sys-color-on-surface); }
.create-form { display: flex; gap: 0.75rem; }
.create-form input { flex: 1; padding: 0.6rem 1rem; border: 1px solid var(--md-sys-color-outline-variant); border-radius: var(--md-sys-shape-sm); font-size: 0.95rem; }
.create-form button { padding: 0.6rem 1.5rem; background: var(--md-sys-color-primary); color: var(--md-sys-color-on-primary); border: none; border-radius: var(--md-sys-shape-sm); cursor: pointer; }
.content-layout { display: flex; gap: 2rem; }
.session-list { width: 350px; flex-shrink: 0; background: var(--md-sys-color-surface); padding: 1.5rem; border-radius: var(--md-sys-shape-md); box-shadow: var(--md-sys-elevation-1); }
.session-list h3 { margin-bottom: 1rem; color: var(--md-sys-color-on-surface); }
.session-item { display: flex; justify-content: space-between; align-items: center; padding: 0.75rem; border-radius: var(--md-sys-shape-sm); cursor: pointer; margin-bottom: 0.25rem; }
.session-item:hover { background: var(--md-sys-color-surface-variant); }
.session-item.active { background: var(--md-sys-color-primary-container); }
.session-info { display: flex; flex-direction: column; }
.session-title { font-weight: 600; color: var(--md-sys-color-on-surface); }
.session-agent { font-size: 0.8rem; color: var(--md-sys-color-on-surface-variant); }
.status-badge { padding: 0.2rem 0.5rem; border-radius: var(--md-sys-shape-xl); font-size: 0.75rem; font-weight: 600; }
.status-badge.active { background: var(--md-sys-color-secondary-container); color: var(--md-sys-color-on-secondary-container); }
.status-badge.completed { background: var(--md-sys-color-primary-container); color: var(--md-sys-color-on-primary-container); }
.status-badge.failed { background: var(--md-sys-color-error-container); color: var(--md-sys-color-on-error-container); }
.status-badge.paused { background: var(--md-sys-color-tertiary-container); color: var(--md-sys-color-on-tertiary-container); }
.session-detail { flex: 1; background: var(--md-sys-color-surface); padding: 2rem; border-radius: var(--md-sys-shape-md); box-shadow: var(--md-sys-elevation-1); }
.session-detail h2 { color: var(--md-sys-color-on-surface); margin-bottom: 1rem; }
.detail-meta { display: flex; gap: 0.5rem; margin-bottom: 1.5rem; }
.badge { padding: 0.3rem 0.75rem; background: var(--md-sys-color-surface-variant); color: var(--md-sys-color-on-surface-variant); border-radius: var(--md-sys-shape-xl); font-size: 0.8rem; }
.status-active { background: var(--md-sys-color-secondary-container); color: var(--md-sys-color-on-secondary-container); }
.status-completed { background: var(--md-sys-color-primary-container); color: var(--md-sys-color-on-primary-container); }
.status-failed { background: var(--md-sys-color-error-container); color: var(--md-sys-color-on-error-container); }
.timeline { margin-bottom: 1.5rem; }
.timeline-item { display: flex; padding: 0.5rem 0; border-bottom: 1px solid var(--md-sys-color-surface-variant); }
.timeline-item .label { font-weight: 600; color: var(--md-sys-color-on-surface); min-width: 100px; }
.timeline-item .value { color: var(--md-sys-color-on-surface-variant); }
.tags { display: flex; gap: 0.5rem; margin-bottom: 1.5rem; }
.tag { padding: 0.2rem 0.5rem; background: var(--md-sys-color-surface-variant); color: var(--md-sys-color-on-surface-variant); border-radius: var(--md-sys-shape-xs); font-size: 0.8rem; }
.delete-btn { padding: 0.5rem 1rem; background: var(--md-sys-color-error); color: var(--md-sys-color-on-error); border: none; border-radius: var(--md-sys-shape-xs); cursor: pointer; }
/* Responsive */
@media (max-width: 900px) {
  .session-manager { padding: 1.25rem; }
  .create-form { flex-direction: column; }
  .create-form input { width: 100%; }
  .create-form button { width: 100%; }
  .content-layout { flex-direction: column; }
  .session-list { width: 100%; }
}
@media (max-width: 600px) {
  .session-manager { padding: 1rem; }
}
</style>
