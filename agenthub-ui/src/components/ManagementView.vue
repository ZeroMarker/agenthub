<script setup lang="ts">
import { onMounted, ref } from 'vue'
import PageHeader from './common/PageHeader.vue'
import { useTauriApi } from '../composables/useTauriApi'
import type { StatusOverview, AuditInfo } from '../types'

const api = useTauriApi()

const overview = ref<StatusOverview | null>(null)
const loadingOverview = ref(false)

const auditEvents = ref<AuditInfo[]>([])
const auditAction = ref('')
const auditTarget = ref('')
const auditDays = ref<number | null>(null)
const auditLimit = ref(50)
const loadingAudit = ref(false)

const backupPath = ref('')
const restorePath = ref('')
const backupResult = ref<string>('')
const restoreResult = ref<string>('')
const busy = ref(false)

const error = ref('')
const notice = ref('')

onMounted(loadAll)

async function loadAll() {
  await Promise.all([loadOverview(), loadAudit()])
}

async function loadOverview() {
  loadingOverview.value = true
  try {
    overview.value = await api.getStatusOverview()
  } catch (err) {
    error.value = `Failed to load status: ${err}`
  } finally {
    loadingOverview.value = false
  }
}

async function loadAudit() {
  loadingAudit.value = true
  try {
    auditEvents.value = await api.listAudit(
      auditAction.value || null,
      auditTarget.value || null,
      auditDays.value,
      auditLimit.value,
    )
  } catch (err) {
    error.value = `Failed to load audit log: ${err}`
  } finally {
    loadingAudit.value = false
  }
}

async function clearAudit() {
  if (!confirm('Clear the entire audit log? This cannot be undone.')) return
  try {
    await api.clearAudit()
    notice.value = 'Audit log cleared'
    await loadAudit()
  } catch (err) {
    error.value = `Failed to clear audit log: ${err}`
  }
}

async function createBackup() {
  busy.value = true
  backupResult.value = ''
  error.value = ''
  try {
    const path = backupPath.value.trim() || `agenthub-backup-${new Date().toISOString().replace(/[:.]/g, '-').slice(0, 19)}.json`
    const manifest = await api.createBackup(path)
    backupResult.value = `✅ Backup written to ${path} — configs ${manifest.counts.configs} · prompts ${manifest.counts.prompts} (+${manifest.counts.prompt_versions} versions) · sessions ${manifest.counts.sessions} · templates ${manifest.counts.session_templates} · memories ${manifest.counts.memories} · audit ${manifest.counts.audit_events}`
    await loadOverview()
    await loadAudit()
  } catch (err) {
    error.value = `Backup failed: ${err}`
  } finally {
    busy.value = false
  }
}

async function restoreBackup() {
  if (!restorePath.value.trim()) {
    error.value = 'Enter a backup file path to restore'
    return
  }
  if (!confirm(`Restore from ${restorePath.value}? Existing data will be overwritten.`)) return
  busy.value = true
  restoreResult.value = ''
  error.value = ''
  try {
    const manifest = await api.restoreBackup(restorePath.value.trim())
    restoreResult.value = `✅ Restored from ${restorePath.value} — configs ${manifest.counts.configs} · prompts ${manifest.counts.prompts} · sessions ${manifest.counts.sessions} · memories ${manifest.counts.memories}`
    await loadAll()
  } catch (err) {
    error.value = `Restore failed: ${err}`
  } finally {
    busy.value = false
  }
}

function fmtTime(ts: string): string {
  return new Date(ts).toLocaleString()
}
</script>

<template>
  <div class="management-view">
    <PageHeader title="Management" subtitle="Workspace overview, audit log and backup/restore" />

    <div v-if="error" class="error-banner">
      <span>{{ error }}</span>
      <button class="error-dismiss" aria-label="Dismiss" @click="error = ''">✕</button>
    </div>
    <div v-if="notice" class="notice-banner">
      <span>{{ notice }}</span>
      <button class="error-dismiss" aria-label="Dismiss" @click="notice = ''">✕</button>
    </div>

    <!-- Status overview -->
    <section class="overview-section" aria-label="Status overview">
      <div class="section-head">
        <h2 class="section-title">仪表盘 Dashboard</h2>
        <button class="m3-btn-tonal" :disabled="loadingOverview" @click="loadOverview">
          {{ loadingOverview ? 'Refreshing…' : '🔄 Refresh' }}
        </button>
      </div>

      <div v-if="overview" class="stat-grid">
        <div class="stat-card m3-surface">
          <span class="stat-icon">📦</span>
          <span class="stat-value">{{ overview.catalog.total }}</span>
          <span class="stat-label">Catalog agents</span>
          <span class="stat-sub">{{ overview.catalog.cli }} CLI · {{ overview.catalog.desktop }} Desktop</span>
        </div>
        <div class="stat-card m3-surface">
          <span class="stat-icon">✅</span>
          <span class="stat-value">{{ overview.installed_agents }}</span>
          <span class="stat-label">Installed</span>
          <span class="stat-sub">{{ overview.platform }}</span>
        </div>
        <div class="stat-card m3-surface">
          <span class="stat-icon">⚙️</span>
          <span class="stat-value">{{ overview.configs }}</span>
          <span class="stat-label">Configs</span>
          <span class="stat-sub">agents configured</span>
        </div>
        <div class="stat-card m3-surface">
          <span class="stat-icon">📝</span>
          <span class="stat-value">{{ overview.prompts }}</span>
          <span class="stat-label">Prompts</span>
          <span class="stat-sub">templates</span>
        </div>
        <div class="stat-card m3-surface">
          <span class="stat-icon">💬</span>
          <span class="stat-value">{{ overview.sessions.total }}</span>
          <span class="stat-label">Sessions</span>
          <span class="stat-sub">{{ overview.sessions.active }} active · {{ overview.sessions.completed }} completed</span>
        </div>
        <div class="stat-card m3-surface">
          <span class="stat-icon">💰</span>
          <span class="stat-value">{{ overview.sessions.total_tokens.toLocaleString() }}</span>
          <span class="stat-label">Tokens</span>
          <span class="stat-sub">${{ overview.sessions.total_cost.toFixed(4) }} estimated</span>
        </div>
        <div class="stat-card m3-surface">
          <span class="stat-icon">🧠</span>
          <span class="stat-value">{{ overview.memories.total }}</span>
          <span class="stat-label">Memories</span>
          <span class="stat-sub">{{ overview.memories.decayed }} decayed</span>
        </div>
        <div class="stat-card m3-surface">
          <span class="stat-icon">🛠️</span>
          <span class="stat-value">{{ overview.skills_total }}</span>
          <span class="stat-label">Skills</span>
          <span class="stat-sub">{{ overview.skills_enabled }} enabled</span>
        </div>
        <div class="stat-card m3-surface">
          <span class="stat-icon">📜</span>
          <span class="stat-value">{{ overview.audit_events }}</span>
          <span class="stat-label">Audit events</span>
          <span class="stat-sub">v{{ overview.agenthub_version }}</span>
        </div>
      </div>
      <div v-else-if="loadingOverview" class="loading-hint">Loading overview…</div>
    </section>

    <!-- Backup / restore -->
    <section class="backup-section" aria-label="Backup and restore">
      <h2 class="section-title">备份 / 恢复 Backup &amp; Restore</h2>
      <div class="backup-row">
        <input
          v-model="backupPath"
          class="path-input"
          placeholder="Output path (default: agenthub-backup-<timestamp>.json)"
          aria-label="Backup output path"
          @keydown.enter="createBackup"
        />
        <button class="m3-btn-filled" :disabled="busy" @click="createBackup">💾 Create Backup</button>
      </div>
      <div class="backup-row">
        <input
          v-model="restorePath"
          class="path-input"
          placeholder="Path to backup file to restore"
          aria-label="Restore input path"
          @keydown.enter="restoreBackup"
        />
        <button class="m3-btn-tonal" :disabled="busy" @click="restoreBackup">♻️ Restore</button>
      </div>
      <p v-if="backupResult" class="result-line success">{{ backupResult }}</p>
      <p v-if="restoreResult" class="result-line success">{{ restoreResult }}</p>
    </section>

    <!-- Audit log -->
    <section class="audit-section" aria-label="Audit log">
      <div class="section-head">
        <h2 class="section-title">审计日志 Audit Log</h2>
        <button class="m3-btn-outlined" @click="clearAudit">🗑️ Clear</button>
      </div>

      <div class="filter-row">
        <input
          v-model="auditAction"
          class="filter-input"
          placeholder="Action filter (e.g. install)"
          aria-label="Filter by action"
          @input="loadAudit"
        />
        <input
          v-model="auditTarget"
          class="filter-input"
          placeholder="Target filter (e.g. agent id)"
          aria-label="Filter by target"
          @input="loadAudit"
        />
        <select v-model.number="auditDays" class="filter-select" aria-label="Time range" @change="loadAudit">
          <option :value="null">Any time</option>
          <option :value="1">Last 1 day</option>
          <option :value="7">Last 7 days</option>
          <option :value="30">Last 30 days</option>
          <option :value="90">Last 90 days</option>
        </select>
        <select v-model.number="auditLimit" class="filter-select" aria-label="Limit" @change="loadAudit">
          <option :value="20">20</option>
          <option :value="50">50</option>
          <option :value="100">100</option>
          <option :value="500">500</option>
        </select>
      </div>

      <div v-if="loadingAudit" class="loading-hint">Loading audit log…</div>

      <div v-else-if="auditEvents.length === 0" class="empty-hint">No audit events found.</div>

      <div v-else class="audit-table-wrap">
        <table class="audit-table">
          <thead>
            <tr>
              <th scope="col">Timestamp</th>
              <th scope="col">Status</th>
              <th scope="col">Action</th>
              <th scope="col">Target</th>
              <th scope="col">Details</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="event in auditEvents" :key="event.id">
              <td class="cell-time">{{ fmtTime(event.timestamp) }}</td>
              <td>
                <span :class="['status-pill', event.success ? 'ok' : 'fail']">
                  {{ event.success ? 'ok' : 'FAIL' }}
                </span>
              </td>
              <td class="cell-action">{{ event.action }}</td>
              <td class="cell-target">{{ event.target }}</td>
              <td class="cell-details">{{ event.details || '' }}</td>
            </tr>
          </tbody>
        </table>
      </div>
    </section>
  </div>
</template>

<style scoped>
.management-view {
  padding: 2rem;
  display: flex;
  flex-direction: column;
  gap: 2rem;
}

.error-banner,
.notice-banner {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0.75rem 1rem;
  border-radius: var(--md-sys-shape-md);
  font: var(--md-sys-typescale-body-medium);
}
.error-banner {
  background: var(--md-sys-color-error-container);
  color: var(--md-sys-color-on-error-container);
}
.notice-banner {
  background: var(--md-sys-color-secondary-container);
  color: var(--md-sys-color-on-secondary-container);
}
.error-dismiss {
  background: none;
  border: none;
  color: inherit;
  cursor: pointer;
  font-size: 1rem;
}

.section-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 1rem;
}
.section-title {
  font: var(--md-sys-typescale-title-large);
  color: var(--md-sys-color-on-surface);
}
.section-head .m3-btn-outlined {
  font-size: 0.875rem;
  padding: 0.5rem 1rem;
}

.stat-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
  gap: 1rem;
}
.stat-card {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  padding: 1.25rem;
  border-radius: var(--md-sys-shape-md);
}
.stat-icon { font-size: 1.5rem; }
.stat-value {
  font: var(--md-sys-typescale-headline-small);
  color: var(--md-sys-color-on-surface);
}
.stat-label {
  font: var(--md-sys-typescale-label-large);
  color: var(--md-sys-color-primary);
}
.stat-sub {
  font: var(--md-sys-typescale-body-small);
  color: var(--md-sys-color-on-surface-variant);
}

.backup-section,
.audit-section,
.overview-section {
  background: var(--md-sys-color-surface);
  padding: 1.5rem;
  border-radius: var(--md-sys-shape-lg);
  border: 1px solid var(--md-sys-color-outline-variant);
}

.backup-row {
  display: flex;
  gap: 0.75rem;
  margin-bottom: 0.75rem;
}
.path-input,
.filter-input {
  flex: 1;
  padding: 0.625rem 1rem;
  border: 1px solid var(--md-sys-color-outline-variant);
  border-radius: var(--md-sys-shape-full);
  background: var(--md-sys-color-surface-container-highest);
  color: var(--md-sys-color-on-surface);
  font: var(--md-sys-typescale-body-medium);
}
.path-input:focus,
.filter-input:focus {
  outline: 2px solid var(--md-sys-color-primary);
  outline-offset: 1px;
}
.filter-row {
  display: flex;
  gap: 0.5rem;
  margin-bottom: 1rem;
  flex-wrap: wrap;
}
.filter-select {
  padding: 0.5rem 0.75rem;
  border: 1px solid var(--md-sys-color-outline-variant);
  border-radius: var(--md-sys-shape-full);
  background: var(--md-sys-color-surface-container-highest);
  color: var(--md-sys-color-on-surface);
  font: var(--md-sys-typescale-body-medium);
}

.result-line {
  font: var(--md-sys-typescale-body-medium);
  margin-top: 0.5rem;
}
.result-line.success {
  color: var(--md-sys-color-primary);
}

.audit-table-wrap {
  overflow-x: auto;
  border-radius: var(--md-sys-shape-md);
  border: 1px solid var(--md-sys-color-outline-variant);
}
.audit-table {
  width: 100%;
  border-collapse: collapse;
  font: var(--md-sys-typescale-body-medium);
}
.audit-table thead th {
  text-align: left;
  padding: 0.75rem 1rem;
  background: var(--md-sys-color-surface-variant);
  color: var(--md-sys-color-on-surface-variant);
  font: var(--md-sys-typescale-label-large);
  position: sticky;
  top: 0;
}
.audit-table tbody td {
  padding: 0.625rem 1rem;
  border-top: 1px solid var(--md-sys-color-outline-variant);
  color: var(--md-sys-color-on-surface);
  vertical-align: top;
}
.cell-time { white-space: nowrap; font-variant-numeric: tabular-nums; }
.cell-action { font-weight: 600; white-space: nowrap; }
.cell-target { word-break: break-all; }
.cell-details { color: var(--md-sys-color-on-surface-variant); }

.status-pill {
  display: inline-block;
  padding: 0.125rem 0.625rem;
  border-radius: var(--md-sys-shape-full);
  font: var(--md-sys-typescale-label-medium);
}
.status-pill.ok {
  background: var(--md-sys-color-secondary-container);
  color: var(--md-sys-color-on-secondary-container);
}
.status-pill.fail {
  background: var(--md-sys-color-error-container);
  color: var(--md-sys-color-on-error-container);
}

.loading-hint,
.empty-hint {
  padding: 1rem;
  color: var(--md-sys-color-on-surface-variant);
  font: var(--md-sys-typescale-body-medium);
}

@media (max-width: 900px) {
  .management-view { padding: 1.25rem; }
  .backup-row { flex-direction: column; }
}
@media (max-width: 600px) {
  .management-view { padding: 1rem; }
  .stat-grid { grid-template-columns: repeat(auto-fill, minmax(140px, 1fr)); }
}
</style>
