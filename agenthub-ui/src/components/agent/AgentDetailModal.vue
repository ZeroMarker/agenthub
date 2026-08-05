<script setup lang="ts">
interface InstallerInfo { platform: string; manager: string; package: string | null }
interface Agent {
  id: string; name: string; description: string
  kind: 'CLI' | 'Desktop'; provider: string; homepage: string; status: string
  installers: InstallerInfo[]
  catalog_verified_at: string | null; installer_verified_at: string | null
}

defineProps<{ agent: Agent; loading: boolean }>()
const emit = defineEmits<{ install: [name: string]; uninstall: [name: string]; close: [] }>()
</script>

<template>
  <div>
    <div class="detail-badges">
      <span :class="['kind-chip', agent.kind.toLowerCase()]">{{ agent.kind }}</span>
      <span :class="['status-chip', agent.status]">{{ agent.status }}</span>
    </div>
    <p class="detail-desc">{{ agent.description }}</p>

    <div class="detail-grid">
      <div class="detail-row"><span class="label">Provider</span><span class="value">{{ agent.provider }}</span></div>
      <div class="detail-row"><span class="label">Homepage</span><a :href="agent.homepage" target="_blank" class="value link">{{ agent.homepage }}</a></div>
      <div class="detail-row"><span class="label">ID</span><code class="value">{{ agent.id }}</code></div>
      <div v-if="agent.catalog_verified_at" class="detail-row"><span class="label">Catalog Verified</span><span class="value">{{ agent.catalog_verified_at }}</span></div>
      <div v-if="agent.installer_verified_at" class="detail-row"><span class="label">Installer Verified</span><span class="value">{{ agent.installer_verified_at }}</span></div>
    </div>

    <h3 class="section-title">Platform Installers</h3>
    <div class="installer-list">
      <div v-for="inst in agent.installers" :key="inst.platform" class="installer-item">
        <span class="inst-platform">{{ inst.platform }}</span>
        <span class="inst-manager">{{ inst.manager }}</span>
        <code v-if="inst.package" class="inst-pkg">{{ inst.package }}</code>
        <span v-else class="inst-manual">Manual</span>
      </div>
    </div>

    <div class="modal-footer">
      <button class="m3-btn-tonal" @click="emit('install', agent.id)" :disabled="loading">Install</button>
      <button class="m3-btn-outlined" @click="emit('uninstall', agent.id)" :disabled="loading">Uninstall</button>
      <button class="m3-btn-outlined" @click="emit('close')">Close</button>
    </div>
  </div>
</template>

<style scoped>
.detail-badges { display: flex; gap: 0.5rem; margin-bottom: 1rem; }
.kind-chip { font: var(--md-sys-typescale-label-small); padding: 0.125rem 0.625rem; border-radius: var(--md-sys-shape-full); text-transform: uppercase; font-weight: 600; letter-spacing: 0.5px; }
.kind-chip.cli { background: var(--md-sys-color-secondary-container); color: var(--md-sys-color-on-secondary-container); }
.kind-chip.desktop { background: var(--md-sys-color-tertiary-container); color: var(--md-sys-color-on-tertiary-container); }
.status-chip { font: var(--md-sys-typescale-label-small); padding: 0.125rem 0.625rem; border-radius: var(--md-sys-shape-full); }
.status-chip.verified, .status-chip.installed { background: var(--md-sys-color-secondary-container); color: var(--md-sys-color-on-secondary-container); }
.status-chip.community { background: var(--md-sys-color-tertiary-container); color: var(--md-sys-color-on-tertiary-container); }
.status-chip.deprecated { background: var(--md-sys-color-error-container); color: var(--md-sys-color-on-error-container); }
.status-chip.manual { background: var(--md-sys-color-surface-variant); color: var(--md-sys-color-on-surface-variant); }
.detail-desc { font: var(--md-sys-typescale-body-large); color: var(--md-sys-color-on-surface-variant); margin-bottom: 1.5rem; }
.detail-grid { display: flex; flex-direction: column; gap: 0.75rem; margin-bottom: 1.5rem; }
.detail-row { display: flex; align-items: center; gap: 1rem; }
.label { font: var(--md-sys-typescale-label-large); color: var(--md-sys-color-on-surface); min-width: 120px; }
.value { font: var(--md-sys-typescale-body-medium); color: var(--md-sys-color-on-surface-variant); }
.value.link { color: var(--md-sys-color-primary); text-decoration: none; }
.value.link:hover { text-decoration: underline; }
.section-title { font: var(--md-sys-typescale-title-medium); color: var(--md-sys-color-on-surface); margin-bottom: 0.75rem; margin-top: 1.5rem; }
.installer-list { display: flex; flex-direction: column; gap: 0.375rem; margin-bottom: 1.5rem; }
.installer-item { display: flex; align-items: center; gap: 0.75rem; padding: 0.5rem; background: var(--md-sys-color-surface-variant); border-radius: var(--md-sys-shape-sm); }
.inst-platform { font: var(--md-sys-typescale-label-large); color: var(--md-sys-color-on-surface); min-width: 80px; }
.inst-manager { font: var(--md-sys-typescale-label-small); padding: 0.125rem 0.5rem; border-radius: var(--md-sys-shape-xs); background: var(--md-sys-color-secondary-container); color: var(--md-sys-color-on-secondary-container); }
.inst-pkg { font-size: 0.85em; }
.inst-manual { font-style: italic; color: var(--md-sys-color-on-surface-variant); opacity: 0.7; }
.modal-footer { display: flex; justify-content: flex-end; gap: 0.75rem; padding-top: 1.5rem; border-top: 1px solid var(--md-sys-color-outline-variant); }
</style>
