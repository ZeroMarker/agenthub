<script setup lang="ts">
withDefaults(defineProps<{
  text?: string
}>(), {
  text: 'Loading...',
})
</script>

<template>
  <div class="loading">
    <div class="spinner"></div>
    <p class="loading-text">{{ text }}</p>
  </div>
</template>

<style scoped>
.loading {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 1rem;
  padding: 3rem;
  color: var(--md-sys-color-on-surface-variant);
}
.spinner {
  width: 40px;
  height: 40px;
  border: 3px solid var(--md-sys-color-outline-variant);
  border-top-color: var(--md-sys-color-primary);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}
@keyframes spin { to { transform: rotate(360deg); } }
.loading-text {
  font: var(--md-sys-typescale-body-large);
  color: var(--md-sys-color-on-surface-variant);
}
@media (prefers-reduced-motion: reduce) {
  .spinner { animation-duration: 0s; }
}
</style>
