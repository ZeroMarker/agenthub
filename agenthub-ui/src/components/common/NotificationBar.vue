<script setup lang="ts">
defineProps<{
  message: string
  type: 'success' | 'error'
}>()

defineEmits<{
  close: []
}>()
</script>

<template>
  <div v-if="message" :class="['notification', type]">
    <span class="notification-text">{{ message }}</span>
    <button class="notification-close" @click="$emit('close')" aria-label="Dismiss notification">&times;</button>
  </div>
</template>

<style scoped>
.notification {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0.75rem 1rem;
  border-radius: var(--md-sys-shape-md);
  margin-bottom: 1rem;
  animation: slideDown var(--md-sys-motion-duration-emphasized) var(--md-sys-motion-easing-emphasized);
  font: var(--md-sys-typescale-body-medium);
}
.notification.success {
  background: var(--md-sys-color-secondary-container);
  color: var(--md-sys-color-on-secondary-container);
}
.notification.error {
  background: var(--md-sys-color-error-container);
  color: var(--md-sys-color-on-error-container);
}
.notification-text { flex: 1; }
.notification-close {
  background: none; border: none; font-size: 1.25rem;
  cursor: pointer; opacity: 0.6; padding: 0 0.25rem;
  color: inherit;
}
.notification-close:hover { opacity: 1; }
@keyframes slideDown {
  from { transform: translateY(-8px); opacity: 0; }
  to { transform: translateY(0); opacity: 1; }
}
@media (prefers-reduced-motion: reduce) {
  .notification { animation: none; }
}
</style>
