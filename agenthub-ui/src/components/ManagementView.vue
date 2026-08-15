<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue'
import PageHeader from './common/PageHeader.vue'
import { useTauriApi } from '../composables/useTauriApi'
import type { StatusOverview, AuditInfo, TrendPoint, BudgetReport, MonitorReport } from '../types'

const api = useTauriApi()

const overview = ref<StatusOverview | null>(null)
const loadingOverview = ref(false)

const trend = ref<TrendPoint[]>([])
const trendDays = ref(7)
const maxTrendCost = ref(0)
const maxTrendSessions = ref(0)
const loadingTrend = ref(false)

const budget = ref<BudgetReport | null>(null)
const budgetDaily = ref<number | null>(null)
const budgetMonthly = ref<number | null>(null)

const monitor = ref<MonitorReport | null>(null)
const loadingMonitor = ref(false)

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

let auditTimer: ReturnType<typeof setTimeout> | null = null

onMounted(loadAll)
onUnmounted(() => {
  if (auditTimer) clearTimeout(auditTimer)
})

async function loadAll() {
  await Promise.all([loadOverview(), loadAudit(), loadBudget(), loadTrend()])
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

async function loadBudget() {
  try {
    budget.value = await api.getSessionBudget()
    budgetDaily.value = budget.value.daily_limit_usd
    budgetMonthly.value = budget.value.monthly_limit_usd
  } catch (err) {
    error.value = `Failed to load budget: ${err}`
  }
}

async function saveBudget() {
  // v-model.number yields '' for cleared inputs; normalize to null so the
  // backend's Option<f64> receives a valid value instead of a string.
  const daily = typeof budgetDaily.value === 'number' && !Number.isNaN(budgetDaily.value) ? budgetDaily.value : null
  const monthly = typeof budgetMonthly.value === 'number' && !Number.isNaN(budgetMonthly.value) ? budgetMonthly.value : null
  try {
    budget.value = await api.setSessionBudget(daily, monthly)
    notice.value = 'Budget updated'
  } catch (err) {
    error.value = `Failed to save budget: ${err}`
  }
}

function scheduleAuditReload() {
  if (auditTimer) clearTimeout(auditTimer)
  auditTimer = setTimeout(loadAudit, 300)
}

async function loadTrend() {
  loadingTrend.value = true
  try {
    trend.value = await api.getTrend(trendDays.value)
    maxTrendCost.value = Math.max(...trend.value.map((p) => p.cost_usd), 0.01)
    maxTrendSessions.value = Math.max(...trend.value.map((p) => p.sessions_started), 1)
  } catch (err) {
    error.value = `Failed to load trend: ${err}`
  } finally {
    loadingTrend.value = false
  }
}

async function runMonitor() {
  loadingMonitor.value = true
  try {
    monitor.value = await api.runMonitor()
  } catch (err) {
    error.value = `Monitor failed: ${err}`
  } finally {
    loadingMonitor.value = false
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

    <!-- Budget + monitor -->
    <section class="budget-section" aria-label="Cost budget and monitoring">
      <div class="section-head">
        <h2 class="section-title">预算与监控 Budget &amp; Monitor</h2>
        <button class="m3-btn-tonal" :disabled="loadingMonitor" @click="runMonitor">
          {{ loadingMonitor ? 'Checking…' : '🩺 Run Monitor' }}
        </button>
      </div>

      <div v-if="budget" class="budget-grid">
        <div class="budget-item">
          <span class="budget-label">今日花费</span>
          <span class="budget-value">${{ budget.daily_spent_usd.toFixed(4) }}</span>
          <span class="budget-hint">{{ budget.total_tokens_today.toLocaleString() }} tokens</span>
        </div>
        <div class="budget-item">
          <span class="budget-label">本月花费</span>
          <span class="budget-value">${{ budget.monthly_spent_usd.toFixed(4) }}</span>
        </div>
        <div class="budget-item budget-limits">
          <label class="budget-label" for="budget-daily">每日上限 $</label>
          <input id="budget-daily" v-model.number="budgetDaily" class="budget-input" type="number" min="0" step="0.1" placeholder="unlimited" />
          <label class="budget-label" for="budget-monthly">每月上限 $</label>
          <input id="budget-monthly" v-model.number="budgetMonthly" class="budget-input" type="number" min="0" step="0.1" placeholder="unlimited" />
          <button class="m3-btn-outlined small" @click="saveBudget">💾 Save</button>
        </div>
      </div>
      <div v-if="budget && budget.alerts.length > 0" class="alert-list">
        <div v-for="(alert, i) in budget.alerts" :key="i" class="alert-item">⚠️ {{ alert }}</div>
      </div>

      <div v-if="monitor" class="monitor-panel">
        <div :class="['monitor-status', monitor.healthy ? 'ok' : 'warn']">
          {{ monitor.healthy ? '✅ HEALTHY' : '⚠️ ISSUES FOUND' }}
        </div>
        <div class="monitor-stats">
          已安装 <strong>{{ monitor.installed_agents }}</strong> · 诊断 <strong>{{ monitor.diagnostics_passed }}</strong> passed / <strong>{{ monitor.diagnostics_failed }}</strong> failed ·
          <span v-if="monitor.missing_agents.length">未安装 {{ monitor.missing_agents.length }} 个 verified agent</span>
          <span v-if="monitor.incompatible_skills.length">· 不兼容技能 {{ monitor.incompatible_skills.length }}</span>
        </div>
        <ul v-if="monitor.warnings.length" class="monitor-warnings">
          <li v-for="(w, i) in monitor.warnings" :key="i">{{ w }}</li>
        </ul>
      </div>
    </section>

    <!-- Trend -->
    <section class="trend-section" aria-label="Daily trend">
      <div class="section-head">
        <h2 class="section-title">趋势 Trend</h2>
        <select v-model.number="trendDays" class="filter-select" aria-label="Trend range" @change="loadTrend">
          <option :value="7">Last 7 days</option>
          <option :value="14">Last 14 days</option>
          <option :value="30">Last 30 days</option>
        </select>
      </div>

      <div v-if="loadingTrend" class="loading-hint">Loading trend…</div>
      <div v-else-if="trend.length" class="trend-bars">
        <div v-for="point in trend" :key="point.date" class="trend-col" :title="`${point.date}: ${point.sessions_started} sessions, $${point.cost_usd.toFixed(4)}`">
          <div class="trend-bar-cost" :style="{ height: (point.cost_usd / maxTrendCost * 100) + '%' }"></div>
          <div class="trend-bar-sessions" :style="{ height: (point.sessions_started / maxTrendSessions * 100) + '%' }"></div>
          <span class="trend-date">{{ point.date.slice(5) }}</span>
        </div>
      </div>
      <div v-else class="empty-hint">No activity in the selected range.</div>
      <div class="trend-legend">
        <span><i class="legend-dot cost"></i>成本 (USD)</span>
        <span><i class="legend-dot sessions"></i>会话数</span>
      </div>
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
          @input="scheduleAuditReload"
        />
        <input
          v-model="auditTarget"
          class="filter-input"
          placeholder="Target filter (e.g. agent id)"
          aria-label="Filter by target"
          @input="scheduleAuditReload"
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
.overview-section,
.budget-section,
.trend-section {
  background: var(--md-sys-color-surface);
  padding: 1.5rem;
  border-radius: var(--md-sys-shape-lg);
  border: 1px solid var(--md-sys-color-outline-variant);
}

/* --- Budget & monitor --- */
.budget-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
  gap: 1rem;
  margin-bottom: 0.75rem;
}
.budget-item {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  padding: 1rem;
  border-radius: var(--md-sys-shape-md);
  background: var(--md-sys-color-surface-container-highest);
}
.budget-label {
  font: var(--md-sys-typescale-label-medium);
  color: var(--md-sys-color-on-surface-variant);
}
.budget-value {
  font: var(--md-sys-typescale-title-large);
  color: var(--md-sys-color-on-surface);
}
.budget-hint {
  font: var(--md-sys-typescale-body-small);
  color: var(--md-sys-color-on-surface-variant);
}
.budget-limits {
  flex-direction: row;
  align-items: center;
  flex-wrap: wrap;
  gap: 0.5rem;
}
.budget-input {
  width: 7rem;
  padding: 0.375rem 0.5rem;
  border: 1px solid var(--md-sys-color-outline-variant);
  border-radius: var(--md-sys-shape-full);
  background: var(--md-sys-color-surface);
  color: var(--md-sys-color-on-surface);
}
.m3-btn-outlined.small {
  padding: 0.375rem 0.875rem;
  font-size: 0.875rem;
}
.alert-list {
  display: flex;
  flex-direction: column;
  gap: 0.375rem;
  margin-bottom: 0.75rem;
}
.alert-item {
  padding: 0.5rem 0.75rem;
  border-radius: var(--md-sys-shape-sm);
  background: var(--md-sys-color-error-container);
  color: var(--md-sys-color-on-error-container);
  font: var(--md-sys-typescale-body-medium);
}
.monitor-panel {
  margin-top: 0.75rem;
  padding: 1rem;
  border-radius: var(--md-sys-shape-md);
  border: 1px solid var(--md-sys-color-outline-variant);
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}
.monitor-status {
  font: var(--md-sys-typescale-title-medium);
}
.monitor-status.ok { color: var(--md-sys-color-primary); }
.monitor-status.warn { color: var(--md-sys-color-error); }
.monitor-stats {
  font: var(--md-sys-typescale-body-medium);
  color: var(--md-sys-color-on-surface-variant);
}
.monitor-warnings {
  list-style: none;
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}
.monitor-warnings li {
  font: var(--md-sys-typescale-body-small);
  color: var(--md-sys-color-error);
}

/* --- Trend --- */
.trend-bars {
  display: flex;
  align-items: flex-end;
  gap: 6px;
  height: 160px;
  padding: 0.5rem 0.25rem 0;
  border-bottom: 1px solid var(--md-sys-color-outline-variant);
}
.trend-col {
  flex: 1;
  display: flex;
  align-items: flex-end;
  justify-content: center;
  gap: 2px;
  height: 100%;
  position: relative;
}
.trend-bar-cost,
.trend-bar-sessions {
  width: 40%;
  min-height: 2px;
  border-radius: var(--md-sys-shape-xs) var(--md-sys-shape-xs) 0 0;
  transition: height var(--md-sys-motion-duration-emphasized) var(--md-sys-motion-easing-emphasized);
}
.trend-bar-cost {
  background: var(--md-sys-color-primary);
}
.trend-bar-sessions {
  background: var(--md-sys-color-tertiary-container);
}
.trend-date {
  position: absolute;
  bottom: -1.35rem;
  font: var(--md-sys-typescale-label-small);
  color: var(--md-sys-color-on-surface-variant);
}
.trend-legend {
  display: flex;
  gap: 1rem;
  margin-top: 1.75rem;
  font: var(--md-sys-typescale-label-medium);
  color: var(--md-sys-color-on-surface-variant);
}
.legend-dot {
  display: inline-block;
  width: 0.75rem;
  height: 0.75rem;
  border-radius: var(--md-sys-shape-full);
  margin-right: 0.25rem;
  vertical-align: middle;
}
.legend-dot.cost { background: var(--md-sys-color-primary); }
.legend-dot.sessions { background: var(--md-sys-color-tertiary-container); }

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
