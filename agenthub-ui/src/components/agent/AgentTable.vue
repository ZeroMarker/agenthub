<script setup lang="ts">
import CardProgress from './CardProgress.vue'

interface InstallerInfo { platform: string; manager: string; package: string | null }
interface Agent {
  id: string; name: string; description: string
  kind: 'CLI' | 'Desktop'; provider: string; homepage: string; status: string
  installers: InstallerInfo[]
  catalog_verified_at: string | null; installer_verified_at: string | null
}

const props = defineProps<{
  agents: Agent[]
  selectedAgents: Set<string>
  sortBy: 'name' | 'type' | 'status'
  sortDirection: 'asc' | 'desc'
  loading: boolean
  progress: Record<string, { step: number; total_steps: number; message: string }>
  installedMap: Map<string, { installed: boolean; version: string | null }>
  results: Record<string, InstallResult | null>
}>()

const emit = defineEmits<{
  toggleSort: [field: 'name' | 'type' | 'status']
  toggleSelect: [id: string]
  selectAll: []
  install: [name: string]
  uninstall: [name: string]
  cancel: [name: string]
}>()

function getSortIcon(field: string): string {
  if (props.sortBy !== field) return '↕'
  return props.sortDirection === 'asc' ? '↑' : '↓'
}
function getInstallerSummary(agent: Agent): string {
  return [...new Set(agent.installers.map(i => i.manager))].join(', ') || 'N/A'
}

interface InstallResult {
  success: boolean
  message: string
  agent_name: string
  command: string
  exit_code: number | null
  stdout: string
  stderr: string
  duration_ms: number
  timed_out: boolean
}

const isCancelled = (r: InstallResult) =>
  !r.success && (r.message === 'Operation cancelled' || r.stderr.includes('cancelled'))
</script>

<template>
  <div class="table-wrap">
    <table class="m3-table">
      <thead>
        <tr>
          <th class="chk">
            <input type="checkbox" :checked="selectedAgents.size === agents.length && agents.length > 0" @change="emit('selectAll')" />
          </th>
          <th class="sortable" @click="emit('toggleSort', 'name')">Name {{ getSortIcon('name') }}</th>
          <th class="sortable" @click="emit('toggleSort', 'type')">Type {{ getSortIcon('type') }}</th>
          <th>Description</th>
          <th>Provider</th>
          <th>Installers</th>
          <th class="sortable" @click="emit('toggleSort', 'status')">Status {{ getSortIcon('status') }}</th>
          <th>Actions</th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="agent in agents" :key="agent.id">
          <td class="chk">
            <input type="checkbox" :checked="selectedAgents.has(agent.id)" @change="emit('toggleSelect', agent.id)" />
          </td>
          <td class="name">{{ agent.name }}</td>
          <td><span :class="['kind-chip', agent.kind.toLowerCase()]">{{ agent.kind }}</span></td>
          <td class="desc">{{ agent.description }}</td>
          <td>{{ agent.provider }}</td>
          <td><code>{{ getInstallerSummary(agent) }}</code></td>
          <td><span :class="['status-chip', agent.status]">{{ agent.status }}</span></td>
          <td>
            <div v-if="progress[agent.id]">
              <CardProgress :progress="progress[agent.id]" />
              <button class="m3-btn-outlined btn-sm" @click="emit('cancel', agent.id)">Cancel</button>
            </div>
            <div v-else-if="results[agent.id] && !results[agent.id]!.success" class="table-op">
              <span :class="['op-badge', isCancelled(results[agent.id]!) ? 'op-cancelled' : 'op-failed']">
                {{ isCancelled(results[agent.id]!) ? 'Cancelled' : 'Failed' }}
              </span>
              <div class="op-actions">
                <button class="m3-btn-tonal btn-sm" @click="emit('install', agent.id)">
                  {{ isCancelled(results[agent.id]!) ? 'Install' : 'Retry' }}
                </button>
                <details class="op-details">
                  <summary>Details</summary>
                  <pre class="op-pre">{{ results[agent.id]!.message }}<template v-if="results[agent.id]!.command">
$ {{ results[agent.id]!.command }}</template><template v-if="results[agent.id]!.stderr">
{{ results[agent.id]!.stderr }}</template></pre>
                </details>
              </div>
            </div>
            <div v-else class="table-actions">
              <button v-if="installedMap.get(agent.id)?.installed" class="m3-btn-tonal btn-sm" @click="emit('uninstall', agent.id)">Uninstall</button>
              <button v-else class="m3-btn-tonal btn-sm" @click="emit('install', agent.id)">Install</button>
              <span v-if="installedMap.get(agent.id)?.version" class="version-chip">v{{ installedMap.get(agent.id)!.version }}</span>
            </div>
          </td>
        </tr>
      </tbody>
    </table>
  </div>
</template>

<style scoped>
.table-wrap {
  background: var(--md-sys-color-surface);
  border-radius: var(--md-sys-shape-expressive-md);
  box-shadow: var(--md-sys-elevation-1);
  overflow-x: auto;
}
.m3-table { width: 100%; border-collapse: collapse; font: var(--md-sys-typescale-body-medium); }
.m3-table th, .m3-table td { padding: 0.875rem 1rem; text-align: left; border-bottom: 1px solid var(--md-sys-color-outline-variant); }
.m3-table th {
  background: var(--md-sys-color-surface-variant);
  color: var(--md-sys-color-on-surface-variant);
  font: var(--md-sys-typescale-label-large);
  position: sticky; top: 0;
}
.m3-table th.sortable { cursor: pointer; user-select: none; }
.m3-table th.sortable:hover { background: color-mix(in srgb, var(--md-sys-color-surface-variant) 80%, var(--md-sys-color-on-surface)); }
.m3-table tbody tr:hover { background: color-mix(in srgb, var(--md-sys-color-surface) 95%, var(--md-sys-color-primary)); }
.chk { width: 48px; text-align: center; }
.chk input { accent-color: var(--md-sys-color-primary); }
.name { font: var(--md-sys-typescale-title-small); color: var(--md-sys-color-on-surface); }
.desc { color: var(--md-sys-color-on-surface-variant); max-width: 260px; }
code { font-size: 0.85em; background: var(--md-sys-color-surface-variant); padding: 2px 6px; border-radius: var(--md-sys-shape-xs); }
.kind-chip { font: var(--md-sys-typescale-label-small); padding: 0.125rem 0.5rem; border-radius: var(--md-sys-shape-full); text-transform: uppercase; font-weight: 600; letter-spacing: 0.5px; }
.kind-chip.cli { background: var(--md-sys-color-secondary-container); color: var(--md-sys-color-on-secondary-container); }
.kind-chip.desktop { background: var(--md-sys-color-tertiary-container); color: var(--md-sys-color-on-tertiary-container); }
.status-chip { font: var(--md-sys-typescale-label-small); padding: 0.125rem 0.5rem; border-radius: var(--md-sys-shape-full); }
.status-chip.verified, .status-chip.installed { background: var(--md-sys-color-secondary-container); color: var(--md-sys-color-on-secondary-container); }
.status-chip.community, .status-chip.warning { background: var(--md-sys-color-tertiary-container); color: var(--md-sys-color-on-tertiary-container); }
.status-chip.deprecated, .status-chip.failed { background: var(--md-sys-color-error-container); color: var(--md-sys-color-on-error-container); }
.status-chip.manual { background: var(--md-sys-color-surface-variant); color: var(--md-sys-color-on-surface-variant); }
.table-actions { display: flex; gap: 0.375rem; align-items: center; }
.btn-sm { padding: 0.375rem 0.75rem; font-size: 0.8rem; }
.version-chip { font: var(--md-sys-typescale-label-small); padding: 0.125rem 0.5rem; border-radius: var(--md-sys-shape-xs); background: var(--md-sys-color-surface-variant); color: var(--md-sys-color-on-surface-variant); }
.table-op { display: flex; flex-direction: column; gap: 0.25rem; align-items: flex-start; }
.op-badge { font: var(--md-sys-typescale-label-small); font-weight: 600; padding: 0.125rem 0.5rem; border-radius: var(--md-sys-shape-full); text-transform: uppercase; letter-spacing: 0.5px; }
.op-failed { background: var(--md-sys-color-error-container); color: var(--md-sys-color-on-error-container); }
.op-cancelled { background: var(--md-sys-color-surface-variant); color: var(--md-sys-color-on-surface-variant); }
.op-actions { display: flex; gap: 0.375rem; align-items: center; }
.op-details summary { cursor: pointer; font: var(--md-sys-typescale-label-small); color: var(--md-sys-color-on-surface-variant); }
.op-pre { font: var(--md-sys-typescale-body-small); background: var(--md-sys-color-surface-variant); color: var(--md-sys-color-on-surface-variant); border-radius: var(--md-sys-shape-xs); padding: 0.375rem; overflow-x: auto; white-space: pre-wrap; word-break: break-all; max-width: 420px; }
</style>
