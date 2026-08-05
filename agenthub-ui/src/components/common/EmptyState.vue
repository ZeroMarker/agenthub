<script setup lang="ts">
withDefaults(defineProps<{
  text?: string
  icon?: string
}>(), {
  text: 'No data found.',
  icon: '📭',
})
</script>

<template>
  <div class="empty">
    <span class="empty-icon">{{ icon }}</span>
    <p class="empty-text">{{ text }}</p>
  </div>
</template>

<style scoped>
.empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 0.75rem;
  padding: 3rem 1.5rem;
  text-align: center;
}
.empty-icon {
  font-size: 3rem;
  opacity: 0.5;
}
.empty-text {
  font: var(--md-sys-typescale-body-large);
  color: var(--md-sys-color-on-surface-variant);
}
</style>
