<script setup lang="ts">
defineProps<{ count: number; total: number; loading: boolean }>()
const emit = defineEmits<{ selectAll: []; batchInstall: []; batchUninstall: [] }>()
</script>

<template>
  <div v-if="total > 0" class="batch-bar">
    <label class="batch-select">
      <input type="checkbox" :checked="count === total && total > 0" @change="emit('selectAll')" />
      <span>Select all ({{ count }}/{{ total }})</span>
    </label>
    <div class="batch-btns">
      <button class="m3-btn-tonal" @click="emit('batchInstall')" :disabled="loading || count === 0">Install ({{ count }})</button>
      <button class="m3-btn-outlined" @click="emit('batchUninstall')" :disabled="loading || count === 0">Uninstall ({{ count }})</button>
    </div>
  </div>
</template>

<style scoped>
.batch-bar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 1rem;
  padding: 0.75rem 1rem;
  background: var(--md-sys-color-surface);
  border-radius: var(--md-sys-shape-sm);
  box-shadow: var(--md-sys-elevation-1);
  font: var(--md-sys-typescale-body-medium);
}
.batch-select { display: flex; align-items: center; gap: 0.5rem; cursor: pointer; }
.batch-select input { accent-color: var(--md-sys-color-primary); }
.batch-btns { display: flex; gap: 0.5rem; }
@media (max-width: 600px) {
  .batch-bar { flex-direction: column; gap: 0.75rem; }
  .batch-btns { width: 100%; }
  .batch-btns button { flex: 1; }
}
</style>
