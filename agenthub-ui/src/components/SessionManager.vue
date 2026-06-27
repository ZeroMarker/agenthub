<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'

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
const searchQuery = ref('')

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

async function deleteSession(id: string) {
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
    <header class="page-header">
      <h1>Session Manager</h1>
      <p>Track and manage your agent sessions</p>
    </header>

    <div v-if="message" :class="['message', messageType]">{{ message }}</div>

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
        <div v-if="loading && sessions.length === 0" class="loading">Loading...</div>
        <div v-else class="list-items">
          <div
            v-for="session in sessions"
            :key="session.id"
            :class="['session-item', { active: selectedSession?.id === session.id }]"
            @click="selectSession(session)"
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
        <p v-if="sessions.length === 0 && !loading" class="empty">No sessions yet</p>
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
          <button class="delete-btn" @click="deleteSession(selectedSession.id)" :disabled="loading">
            Delete Session
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.session-manager { padding: 2rem; }
.page-header { margin-bottom: 2rem; }
.page-header h1 { font-size: 1.75rem; color: #2c3e50; margin-bottom: 0.5rem; }
.page-header p { color: #666; }
.message { padding: 0.75rem 1rem; border-radius: 8px; margin-bottom: 1rem; }
.message.success { background: #d4edda; color: #155724; }
.message.error { background: #f8d7da; color: #721c24; }
.create-section { background: white; padding: 1.5rem; border-radius: 12px; box-shadow: 0 2px 8px rgba(0,0,0,0.05); margin-bottom: 1.5rem; }
.create-section h3 { margin-bottom: 1rem; color: #2c3e50; }
.create-form { display: flex; gap: 0.75rem; }
.create-form input { flex: 1; padding: 0.6rem 1rem; border: 1px solid #e0e0e0; border-radius: 8px; font-size: 0.95rem; }
.create-form button { padding: 0.6rem 1.5rem; background: #27ae60; color: white; border: none; border-radius: 8px; cursor: pointer; }
.content-layout { display: flex; gap: 2rem; }
.session-list { width: 350px; flex-shrink: 0; background: white; padding: 1.5rem; border-radius: 12px; box-shadow: 0 2px 8px rgba(0,0,0,0.05); }
.session-list h3 { margin-bottom: 1rem; color: #2c3e50; }
.session-item { display: flex; justify-content: space-between; align-items: center; padding: 0.75rem; border-radius: 8px; cursor: pointer; margin-bottom: 0.25rem; }
.session-item:hover { background: #f8f9fa; }
.session-item.active { background: #e3f2fd; }
.session-info { display: flex; flex-direction: column; }
.session-title { font-weight: 600; color: #2c3e50; }
.session-agent { font-size: 0.8rem; color: #999; }
.status-badge { padding: 0.2rem 0.5rem; border-radius: 20px; font-size: 0.75rem; font-weight: 600; }
.status-badge.active { background: #d4edda; color: #155724; }
.status-badge.completed { background: #e3f2fd; color: #1565c0; }
.status-badge.failed { background: #f8d7da; color: #721c24; }
.status-badge.paused { background: #fff3cd; color: #856404; }
.session-detail { flex: 1; background: white; padding: 2rem; border-radius: 12px; box-shadow: 0 2px 8px rgba(0,0,0,0.05); }
.session-detail h2 { color: #2c3e50; margin-bottom: 1rem; }
.detail-meta { display: flex; gap: 0.5rem; margin-bottom: 1.5rem; }
.badge { padding: 0.3rem 0.75rem; background: #f0f0f0; color: #666; border-radius: 20px; font-size: 0.8rem; }
.status-active { background: #d4edda; color: #155724; }
.status-completed { background: #e3f2fd; color: #1565c0; }
.status-failed { background: #f8d7da; color: #721c24; }
.timeline { margin-bottom: 1.5rem; }
.timeline-item { display: flex; padding: 0.5rem 0; border-bottom: 1px solid #f0f0f0; }
.timeline-item .label { font-weight: 600; color: #2c3e50; min-width: 100px; }
.timeline-item .value { color: #666; }
.tags { display: flex; gap: 0.5rem; margin-bottom: 1.5rem; }
.tag { padding: 0.2rem 0.5rem; background: #f0f0f0; color: #666; border-radius: 4px; font-size: 0.8rem; }
.delete-btn { padding: 0.5rem 1rem; background: #e74c3c; color: white; border: none; border-radius: 6px; cursor: pointer; }
.loading, .empty { text-align: center; padding: 2rem; color: #999; }
</style>
