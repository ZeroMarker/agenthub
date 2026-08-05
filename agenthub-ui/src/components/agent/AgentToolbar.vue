<script setup lang="ts">
defineProps<{
  searchQuery: string
  viewMode: 'grid' | 'table'
  sortBy: 'name' | 'type' | 'status'
  sortDirection: 'asc' | 'desc'
  loading: boolean
  activeTab: 'all' | 'cli' | 'desktop'
}>()

const emit = defineEmits<{
  searchUpdate: [value: string]
  search: []
  refresh: []
  toggleSort: [field: 'name' | 'type' | 'status']
  toggleView: [mode: 'grid' | 'table']
  setTab: [tab: 'all' | 'cli' | 'desktop']
}>()
</script>

<template>
  <div class="toolbar">
    <div class="search-bar">
      <input
        :value="searchQuery"
        type="text"
        :placeholder="`Search ${activeTab === 'all' ? 'all' : activeTab} agents...`"
        @input="emit('searchUpdate', ($event.target as HTMLInputElement).value)"
        @keyup.enter="emit('search')"
      />
      <button class="m3-btn-tonal" @click="emit('search')" :disabled="loading">Search</button>
      <button class="m3-btn-outlined" @click="emit('refresh')" :disabled="loading">Refresh</button>
    </div>
    <div class="view-controls">
      <div class="sort-chips">
        <button
          v-for="s in ([{ key: 'name', label: 'Name' }, { key: 'type', label: 'Type' }, { key: 'status', label: 'Status' }] as const)"
          :key="s.key"
          :class="['m3-chip', { active: sortBy === s.key }]"
          @click="emit('toggleSort', s.key)"
        >
          {{ s.label }}
          <span class="sort-arrow">{{ sortBy === s.key ? (sortDirection === 'asc' ? '↑' : '↓') : '↕' }}</span>
        </button>
      </div>
      <div class="view-toggle">
        <button :class="['m3-chip', { active: viewMode === 'grid' }]" @click="emit('toggleView', 'grid')">⊞ Grid</button>
        <button :class="['m3-chip', { active: viewMode === 'table' }]" @click="emit('toggleView', 'table')">☰ Table</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.toolbar {
  display: flex;
  flex-wrap: wrap;
  gap: 0.75rem;
  margin-bottom: 1.5rem;
  align-items: center;
}
.search-bar {
  display: flex;
  gap: 0.5rem;
  flex: 1;
  min-width: 280px;
}
.search-bar input {
  flex: 1;
  padding: 0.625rem 1rem;
  border: 1px solid var(--md-sys-color-outline);
  border-radius: var(--md-sys-shape-sm);
  font: var(--md-sys-typescale-body-medium);
  background: var(--md-sys-color-surface);
  color: var(--md-sys-color-on-surface);
  caret-color: var(--md-sys-color-primary);
  transition: border-color var(--md-sys-motion-duration-short) var(--md-sys-motion-easing-emphasized);
}
.search-bar input:focus {
  outline: none;
  border-color: var(--md-sys-color-primary);
  box-shadow: 0 0 0 3px var(--md-sys-color-primary-container);
}
.search-bar input::placeholder { color: var(--md-sys-color-on-surface-variant); opacity: 0.6; }
.view-controls {
  display: flex;
  gap: 0.5rem;
  align-items: center;
}
.sort-chips {
  display: flex;
  gap: 0.25rem;
}
.sort-arrow { font-size: 0.75rem; }
.view-toggle {
  display: flex;
  gap: 0.25rem;
}
@media (max-width: 768px) {
  .toolbar { flex-direction: column; align-items: stretch; }
  .search-bar { flex-direction: column; min-width: auto; }
  .view-controls { flex-wrap: wrap; justify-content: center; }
}
</style>
