<script setup lang="ts">
defineProps<{
  title: string
  subtitle?: string
}>()
</script>

<template>
  <header class="page-header">
    <h1 class="page-title">{{ title }}</h1>
    <p v-if="subtitle" class="page-subtitle">{{ subtitle }}</p>
    <div class="page-accent"></div>
  </header>
</template>

<style scoped>
.page-header {
  margin-bottom: 1.5rem;
  position: relative;
}
.page-title {
  font: var(--md-sys-typescale-headline-medium);
  color: var(--md-sys-color-on-surface);
  margin-bottom: 0.125rem;
}
.page-subtitle {
  font: var(--md-sys-typescale-body-large);
  color: var(--md-sys-color-on-surface-variant);
}
.page-accent {
  width: 2.5rem;
  height: 4px;
  background: var(--md-sys-color-primary);
  border-radius: var(--md-sys-shape-full);
  margin-top: 0.75rem;
}
</style>
