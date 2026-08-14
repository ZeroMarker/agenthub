<script setup lang="ts">
import CardProgress from './CardProgress.vue'
import StatusBadge from '../common/StatusBadge.vue'

interface InstallerInfo {
  platform: string; manager: string; package: string | null
}
interface Agent {
  id: string; name: string; description: string
  kind: 'CLI' | 'Desktop'; provider: string
  homepage: string; status: string
  installers: InstallerInfo[]
  catalog_verified_at: string | null; installer_verified_at: string | null
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

const props = defineProps<{
  agent: Agent
  isSelected: boolean
  installed: boolean
  version: string | null
  progress: { step: number; total_steps: number; message: string } | null
  result: InstallResult | null
}>()

const emit = defineEmits<{
  toggleSelect: [id: string]
  openDetail: [agent: Agent]
  install: [name: string]
  uninstall: [name: string]
  cancel: [name: string]
}>()

function getInstallerSummary(agent: Agent): string {
  return [...new Set(agent.installers.map(i => i.manager))].join(', ') || 'N/A'
}

const isCancelled = (r: InstallResult) =>
  !r.success && (r.message === 'Operation cancelled' || r.stderr.includes('cancelled'))
</script>

<template>
  <div
    :class="['m3-card', { selected: isSelected }]"
    @click="emit('openDetail', agent)"
    @keydown.enter="emit('openDetail', agent)"
    role="button"
    :aria-label="`View details for ${agent.name}`"
    tabindex="0"
  >
    <div class="card-top">
      <input
        type="checkbox"
        :checked="isSelected"
        @change="emit('toggleSelect', agent.id)"
        @click.stop
      />
      <div class="card-chips">
        <StatusBadge :status="agent.status" />
        <span :class="['kind-chip', agent.kind.toLowerCase()]">{{ agent.kind }}</span>
      </div>
    </div>
    <h3 class="card-title">{{ agent.name }}</h3>
    <p class="card-desc">{{ agent.description }}</p>
    <div class="card-meta">
      <span class="meta-provider">{{ agent.provider }}</span>
      <span class="meta-installers">{{ getInstallerSummary(agent) }}</span>
    </div>

    <!-- In-flight operation: progress + cancel -->
    <div v-if="progress" class="op-panel">
      <CardProgress :progress="progress" />
      <button class="m3-btn-outlined btn-sm" @click.stop="emit('cancel', agent.id)">Cancel</button>
    </div>

    <!-- Failed / cancelled operation: retry + expandable details -->
    <div v-else-if="result && !result.success" class="op-panel">
      <div :class="['op-badge', isCancelled(result) ? 'op-cancelled' : 'op-failed']">
        {{ isCancelled(result) ? 'Cancelled' : 'Failed' }}
      </div>
      <div class="op-message">{{ result.message }}</div>
      <div class="op-actions">
        <button
          v-if="!isCancelled(result)"
          class="m3-btn-tonal btn-sm"
          @click.stop="emit('install', agent.id)"
        >Retry</button>
        <button
          v-else
          class="m3-btn-tonal btn-sm"
          @click.stop="emit('install', agent.id)"
        >Install</button>
      </div>
      <details class="op-details" v-if="result.command || result.stderr || result.stdout">
        <summary>Failure details</summary>
        <pre class="op-pre"><template v-if="result.command">$ {{ result.command }}
</template><template v-if="result.exit_code !== null">exit code: {{ result.exit_code }}
</template><template v-if="result.timed_out">timed out after {{ result.duration_ms }}ms
</template><template v-if="result.stderr">stderr:
{{ result.stderr }}</template><template v-if="result.stdout">
stdout:
{{ result.stdout }}</template></pre>
      </details>
    </div>

    <!-- Normal state: install/uninstall -->
    <div v-else class="card-actions">
      <button v-if="installed" class="m3-btn-tonal" @click.stop="emit('uninstall', agent.id)">Uninstall</button>
      <button v-else class="m3-btn-tonal" @click.stop="emit('install', agent.id)">Install</button>
      <span v-if="version" class="version-chip">v{{ version }}</span>
    </div>
  </div>
</template>

<style scoped>
.m3-card {
  background: var(--md-sys-color-surface);
  border-radius: var(--md-sys-shape-expressive-md);
  padding: 1.25rem;
  box-shadow: var(--md-sys-elevation-1);
  cursor: pointer;
  transition: box-shadow var(--md-sys-motion-duration-emphasized) var(--md-sys-motion-easing-emphasized),
              transform var(--md-sys-motion-duration-spring) var(--md-sys-motion-easing-spring);
  display: flex;
  flex-direction: column;
  gap: 0.625rem;
  outline: 1px solid var(--md-sys-color-outline-variant);
  will-change: transform;
}
.m3-card:hover {
  box-shadow: var(--md-sys-elevation-3);
  transform: translateY(-3px) scale(1.005);
}
.m3-card:active {
  transform: translateY(0) scale(0.99);
  transition-duration: var(--md-sys-motion-duration-short);
}
.m3-card.selected {
  outline: 2px solid var(--md-sys-color-primary);
}
.card-top { display: flex; justify-content: space-between; align-items: center; }
.card-chips { display: flex; gap: 0.375rem; align-items: center; }
.card-top input { width: 18px; height: 18px; accent-color: var(--md-sys-color-primary); }
.card-title { font: var(--md-sys-typescale-title-medium); color: var(--md-sys-color-on-surface); }
.card-desc { font: var(--md-sys-typescale-body-medium); color: var(--md-sys-color-on-surface-variant); display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; }
.card-meta { display: flex; gap: 0.5rem; flex-wrap: wrap; }
.meta-provider, .meta-installers {
  font: var(--md-sys-typescale-label-small);
  padding: 0.125rem 0.5rem;
  border-radius: var(--md-sys-shape-xs);
}
.meta-provider { background: var(--md-sys-color-surface-variant); color: var(--md-sys-color-on-surface-variant); }
.meta-installers { background: var(--md-sys-color-secondary-container); color: var(--md-sys-color-on-secondary-container); }
.card-actions { display: flex; gap: 0.5rem; align-items: center; margin-top: auto; }
.kind-chip {
  font: var(--md-sys-typescale-label-small);
  padding: 0.125rem 0.5rem;
  border-radius: var(--md-sys-shape-full);
  text-transform: uppercase;
  font-weight: 600;
  letter-spacing: 0.5px;
}
.kind-chip.cli { background: var(--md-sys-color-secondary-container); color: var(--md-sys-color-on-secondary-container); }
.kind-chip.desktop { background: var(--md-sys-color-tertiary-container); color: var(--md-sys-color-on-tertiary-container); }
.version-chip {
  font: var(--md-sys-typescale-label-small);
  padding: 0.125rem 0.5rem;
  border-radius: var(--md-sys-shape-xs);
  background: var(--md-sys-color-surface-variant);
  color: var(--md-sys-color-on-surface-variant);
}


/* Operation panel (progress/cancel, failure details) */
.op-panel { display: flex; flex-direction: column; gap: 0.5rem; }
.op-panel .btn-sm { align-self: flex-start; }
.op-badge {
  font: var(--md-sys-typescale-label-small);
  font-weight: 600;
  padding: 0.125rem 0.5rem;
  border-radius: var(--md-sys-shape-full);
  align-self: flex-start;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}
.op-failed { background: var(--md-sys-color-error-container); color: var(--md-sys-color-on-error-container); }
.op-cancelled { background: var(--md-sys-color-surface-variant); color: var(--md-sys-color-on-surface-variant); }
.op-message { font: var(--md-sys-typescale-body-small); color: var(--md-sys-color-on-surface-variant); }
.op-actions { display: flex; gap: 0.5rem; }
.op-details { font: var(--md-sys-typescale-label-small); color: var(--md-sys-color-on-surface-variant); }
.op-details summary { cursor: pointer; }
.op-pre {
  font: var(--md-sys-typescale-body-small);
  background: var(--md-sys-color-surface-variant);
  color: var(--md-sys-color-on-surface-variant);
  border-radius: var(--md-sys-shape-xs);
  padding: 0.5rem;
  overflow-x: auto;
  white-space: pre-wrap;
  word-break: break-all;
}

</style>
