<script setup lang="ts">
defineProps<{
  status: string
}>()

function variant(status: string): string {
  switch (status) {
    case 'verified': case 'installed': return 'positive'
    case 'warning': case 'community': return 'caution'
    case 'deprecated': case 'failed': return 'negative'
    default: return 'neutral'
  }
}
</script>

<template>
  <span :class="['status-badge', variant(status)]">{{ status }}</span>
</template>

<style scoped>
.status-badge {
  display: inline-flex;
  align-items: center;
  padding: 0.125rem 0.625rem;
  border-radius: var(--md-sys-shape-full);
  font: var(--md-sys-typescale-label-small);
  text-transform: capitalize;
}
.status-badge.positive {
  background: var(--md-sys-color-secondary-container);
  color: var(--md-sys-color-on-secondary-container);
}
.status-badge.caution {
  background: var(--md-sys-color-tertiary-container);
  color: var(--md-sys-color-on-tertiary-container);
}
.status-badge.negative {
  background: var(--md-sys-color-error-container);
  color: var(--md-sys-color-on-error-container);
}
.status-badge.neutral {
  background: var(--md-sys-color-surface-variant);
  color: var(--md-sys-color-on-surface-variant);
}
</style>
