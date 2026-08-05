<script setup lang="ts">
defineProps<{ progress: { step: number; total_steps: number; message: string } }>()
</script>

<template>
  <div class="cp">
    <div class="cp-track"><div class="cp-fill" :style="{ width: ((progress.step - 1) / Math.max(progress.total_steps - 1, 1) * 100) + '%' }"></div></div>
    <div class="cp-msg">{{ progress.message }}</div>
  </div>
</template>

<style scoped>
.cp { padding: 0.5rem 0; }
.cp-track { height: 4px; background: var(--md-sys-color-surface-variant); border-radius: var(--md-sys-shape-full); overflow: hidden; margin-bottom: 0.25rem; }
.cp-fill { height: 100%; background: var(--md-sys-color-primary); border-radius: var(--md-sys-shape-full); transition: width 0.3s ease; }
.cp-msg { font: var(--md-sys-typescale-label-small); color: var(--md-sys-color-on-surface-variant); text-align: center; }
</style>
