<script setup lang="ts">
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'

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
    <header class="page-header">
      <h1>Diagnostic Tool</h1>
      <p>Check system health and dependencies</p>
    </header>

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

.page-header {
  margin-bottom: 2rem;
}

.page-header h1 {
  font-size: 1.75rem;
  color: #2c3e50;
  margin-bottom: 0.5rem;
}

.page-header p {
  color: #666;
}

.actions {
  margin-bottom: 2rem;
}

.run-btn {
  padding: 0.75rem 2rem;
  background: linear-gradient(135deg, #3498db, #2980b9);
  color: white;
  border: none;
  border-radius: 8px;
  font-size: 1rem;
  font-weight: 600;
  cursor: pointer;
  transition: transform 0.2s, box-shadow 0.2s;
}

.run-btn:hover:not(:disabled) {
  transform: translateY(-2px);
  box-shadow: 0 4px 12px rgba(52, 152, 219, 0.3);
}

.run-btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.error-message {
  padding: 1rem;
  background: #f8d7da;
  color: #721c24;
  border-radius: 8px;
  margin-bottom: 2rem;
}

.results {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
}

.summary-card {
  background: white;
  padding: 2rem;
  border-radius: 12px;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.05);
}

.summary-card h2 {
  margin-bottom: 1.5rem;
  color: #2c3e50;
}

.summary-stats {
  display: flex;
  gap: 2rem;
}

.stat {
  text-align: center;
  padding: 1rem 2rem;
  border-radius: 12px;
  min-width: 100px;
}

.stat.passed {
  background: #d4edda;
}

.stat.warnings {
  background: #fff3cd;
}

.stat.failed {
  background: #f8d7da;
}

.stat-value {
  display: block;
  font-size: 2rem;
  font-weight: 700;
  color: #2c3e50;
}

.stat-label {
  display: block;
  font-size: 0.85rem;
  color: #666;
  margin-top: 0.25rem;
}

.category-section {
  background: white;
  padding: 1.5rem;
  border-radius: 12px;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.05);
}

.category-title {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  margin-bottom: 1rem;
  color: #2c3e50;
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
  border-radius: 8px;
  background: #f8f9fa;
}

.check-item.passed {
  background: #f8fff8;
}

.check-item.warning {
  background: #fffdf5;
}

.check-item.failed {
  background: #fff8f8;
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
  color: #2c3e50;
}

.check-message {
  font-size: 0.9rem;
  color: #666;
}

.raw-output {
  background: white;
  padding: 1.5rem;
  border-radius: 12px;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.05);
}

.raw-output h3 {
  margin-bottom: 1rem;
  color: #2c3e50;
}

.raw-output pre {
  background: #f8f9fa;
  padding: 1rem;
  border-radius: 8px;
  overflow-x: auto;
  font-size: 0.85rem;
  line-height: 1.5;
}

.empty {
  text-align: center;
  padding: 3rem;
  background: white;
  border-radius: 12px;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.05);
  color: #999;
}
</style>
