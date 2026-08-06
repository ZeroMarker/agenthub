<script setup lang="ts">
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import PageHeader from './common/PageHeader.vue'

interface CheckResult {
  name: string
  category: string
  status: string
  message: string
}

interface DiagnosticResult {
  summary: string
  checks: CheckResult[]
  passed: number
  warnings: number
  failed: number
}

const result = ref<DiagnosticResult | null>(null)
const loading = ref(false)
const error = ref('')

async function runDiagnostics() {
  loading.value = true
  error.value = ''
  try {
    result.value = await invoke<DiagnosticResult>('run_diagnostics')
  } catch (err) {
    error.value = `Diagnostics failed: ${err}`
  } finally {
    loading.value = false
  }
}

function getStatusIcon(status: string): string {
  switch (status) {
    case 'Passed': return '✅'
    case 'Warning': return '⚠️'
    case 'Failed': return '❌'
    case 'Skipped': return '⏭️'
    default: return '❓'
  }
}

function getStatusClass(status: string): string {
  switch (status) {
    case 'Passed': return 'passed'
    case 'Warning': return 'warning'
    case 'Failed': return 'failed'
    case 'Skipped': return 'skipped'
    default: return ''
  }
}

function getCategoryIcon(category: string): string {
  switch (category) {
    case 'system': return '🖥️'
    case 'package_manager': return '📦'
    case 'toolchain': return '🔧'
    case 'catalog': return '📋'
    case 'storage': return '💾'
    case 'connectivity': return '🌐'
    default: return '❓'
  }
}

function groupedChecks(checks: CheckResult[]): Record<string, CheckResult[]> {
  const groups: Record<string, CheckResult[]> = {}
  for (const check of checks) {
    if (!groups[check.category]) {
      groups[check.category] = []
    }
    groups[check.category].push(check)
  }
  return groups
}
</script>

<template>
  <div class="diagnostic-view">
    <PageHeader title="Diagnostic Tool" subtitle="Check system health and dependencies" />

    <div class="actions">
      <button
        class="run-btn"
        @click="runDiagnostics"
        :disabled="loading"
      >
        {{ loading ? 'Running...' : 'Run Diagnostics' }}
      </button>
    </div>

    <div v-if="error" class="error-message">
      {{ error }}
    </div>

    <div v-if="result" class="results">
      <div class="summary-card">
        <h2>Summary</h2>
        <div class="summary-stats">
          <div class="stat passed">
            <span class="stat-value">{{ result.passed }}</span>
            <span class="stat-label">Passed</span>
          </div>
          <div class="stat warnings">
            <span class="stat-value">{{ result.warnings }}</span>
            <span class="stat-label">Warnings</span>
          </div>
          <div class="stat failed">
            <span class="stat-value">{{ result.failed }}</span>
            <span class="stat-label">Failed</span>
          </div>
        </div>
      </div>

      <div v-for="(checks, category) in groupedChecks(result.checks)" :key="category" class="category-section">
        <h3 class="category-title">
          <span class="category-icon">{{ getCategoryIcon(category) }}</span>
          {{ category }}
        </h3>
        <div class="checks-list">
          <div
            v-for="check in checks"
            :key="check.name"
            :class="['check-item', getStatusClass(check.status)]"
          >
            <div class="check-status">
              {{ getStatusIcon(check.status) }}
            </div>
            <div class="check-info">
              <span class="check-name">{{ check.name }}</span>
              <span class="check-message">{{ check.message }}</span>
            </div>
          </div>
        </div>
      </div>

      <div class="raw-output">
        <h3>Raw Output</h3>
        <pre>{{ result.summary }}</pre>
      </div>
    </div>

    <div v-if="!result && !loading && !error" class="empty">
      <p>Click "Run Diagnostics" to check your system</p>
    </div>
  </div>
</template>

<style scoped>
.diagnostic-view {
  padding: 2rem;
}

.actions {
  margin-bottom: 2rem;
}

.run-btn {
  padding: 0.75rem 2rem;
  background: var(--md-sys-color-primary);
  color: var(--md-sys-color-on-primary);
  border: none;
  border-radius: var(--md-sys-shape-sm);
  font-size: 1rem;
  font-weight: 600;
  cursor: pointer;
  transition: transform 0.2s, box-shadow 0.2s;
}

.run-btn:hover:not(:disabled) {
  transform: translateY(-2px);
  box-shadow: var(--md-sys-elevation-2);
}

.run-btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.error-message {
  padding: 1rem;
  background: var(--md-sys-color-error-container);
  color: var(--md-sys-color-on-error-container);
  border-radius: var(--md-sys-shape-sm);
  margin-bottom: 2rem;
}

.results {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
}

.summary-card {
  background: var(--md-sys-color-surface);
  padding: 2rem;
  border-radius: var(--md-sys-shape-md);
  box-shadow: var(--md-sys-elevation-1);
}

.summary-card h2 {
  margin-bottom: 1.5rem;
  color: var(--md-sys-color-on-surface);
}

.summary-stats {
  display: flex;
  gap: 2rem;
}

.stat {
  text-align: center;
  padding: 1rem 2rem;
  border-radius: var(--md-sys-shape-md);
  min-width: 100px;
}

.stat.passed {
  background: var(--md-sys-color-secondary-container);
}

.stat.warnings {
  background: var(--md-sys-color-tertiary-container);
}

.stat.failed {
  background: var(--md-sys-color-error-container);
}

.stat-value {
  display: block;
  font-size: 2rem;
  font-weight: 700;
  color: var(--md-sys-color-on-surface);
}

.stat-label {
  display: block;
  font-size: 0.85rem;
  color: var(--md-sys-color-on-surface-variant);
  margin-top: 0.25rem;
}

.category-section {
  background: var(--md-sys-color-surface);
  padding: 1.5rem;
  border-radius: var(--md-sys-shape-md);
  box-shadow: var(--md-sys-elevation-1);
}

.category-title {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  margin-bottom: 1rem;
  color: var(--md-sys-color-on-surface);
  font-size: 1.1rem;
}

.category-icon {
  font-size: 1.2rem;
}

.checks-list {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.check-item {
  display: flex;
  align-items: center;
  gap: 1rem;
  padding: 0.75rem;
  border-radius: var(--md-sys-shape-sm);
  background: var(--md-sys-color-surface-variant);
}

.check-item.passed {
  background: var(--md-sys-color-secondary-container);
}

.check-item.warning {
  background: var(--md-sys-color-tertiary-container);
}

.check-item.failed {
  background: var(--md-sys-color-error-container);
}

.check-status {
  font-size: 1.2rem;
  min-width: 30px;
  text-align: center;
}

.check-info {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

.check-name {
  font-weight: 600;
  color: var(--md-sys-color-on-surface);
}

.check-message {
  font-size: 0.9rem;
  color: var(--md-sys-color-on-surface-variant);
}

.raw-output {
  background: var(--md-sys-color-surface);
  padding: 1.5rem;
  border-radius: var(--md-sys-shape-md);
  box-shadow: var(--md-sys-elevation-1);
}

.raw-output h3 {
  margin-bottom: 1rem;
  color: var(--md-sys-color-on-surface);
}

.raw-output pre {
  background: var(--md-sys-color-surface-variant);
  padding: 1rem;
  border-radius: var(--md-sys-shape-sm);
  overflow-x: auto;
  font-size: 0.85rem;
  line-height: 1.5;
}

/* Responsive */
@media (max-width: 900px) {
  .diagnostic-view { padding: 1.25rem; }
  .summary-stats { flex-direction: column; gap: 0.75rem; }
  .stat { width: 100%; }
}
@media (max-width: 600px) {
  .diagnostic-view { padding: 1rem; }
}
</style>
