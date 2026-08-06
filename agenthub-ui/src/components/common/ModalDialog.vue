<script setup lang="ts">
import { ref, watch, onMounted, onUnmounted } from 'vue'

defineProps<{
  show: boolean
  title?: string
}>()

const emit = defineEmits<{
  close: []
}>()

const modalRef = ref<HTMLElement | null>(null)
let previousActiveElement: HTMLElement | null = null

function getFocusableElements(el: HTMLElement): HTMLElement[] {
  const selectors = [
    'a[href]', 'button:not([disabled])', 'textarea:not([disabled])',
    'input:not([disabled])', 'select:not([disabled])',
    '[tabindex]:not([tabindex="-1"])',
  ]
  return Array.from(el.querySelectorAll<HTMLElement>(selectors.join(', ')))
}

function handleKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape') { emit('close'); return }
  if (e.key === 'Tab' && modalRef.value) {
    const focusable = getFocusableElements(modalRef.value)
    if (focusable.length === 0) return
    const first = focusable[0], last = focusable[focusable.length - 1]
    if (e.shiftKey) {
      if (document.activeElement === first) { e.preventDefault(); last.focus() }
    } else {
      if (document.activeElement === last) { e.preventDefault(); first.focus() }
    }
  }
}

watch(() => modalRef.value, (el) => {
  if (el) {
    previousActiveElement = document.activeElement as HTMLElement
    const focusable = getFocusableElements(el)
    if (focusable.length > 0) setTimeout(() => focusable[0].focus(), 50)
  }
})

onMounted(() => document.addEventListener('keydown', handleKeydown))
onUnmounted(() => {
  document.removeEventListener('keydown', handleKeydown)
  previousActiveElement?.focus()
})
</script>

<template>
  <Teleport to="body">
    <div v-if="show" class="modal-overlay" @click.self="emit('close')" role="dialog" aria-modal="true">
      <div ref="modalRef" class="modal-content">
        <div v-if="title" class="modal-header">
          <h2>{{ title }}</h2>
          <button class="modal-close" @click="emit('close')" aria-label="Close dialog">&times;</button>
        </div>
        <div class="modal-body">
          <slot />
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.modal-overlay {
  position: fixed; top: 0; left: 0; right: 0; bottom: 0;
  background: rgba(0,0,0,0.38); display: flex;
  align-items: center; justify-content: center; z-index: 1000;
}
.modal-content {
  background: var(--md-sys-color-surface);
  border-radius: var(--md-sys-shape-expressive-xl);
  width: 90%; max-width: 640px;
  max-height: 85vh; overflow-y: auto;
  box-shadow: var(--md-sys-elevation-3);
  animation: modalIn var(--md-sys-motion-duration-emphasized) var(--md-sys-motion-easing-spring);
}
@keyframes modalIn {
  from { transform: translateY(24px) scale(0.97); opacity: 0; }
  to { transform: translateY(0); opacity: 1; }
}
.modal-header {
  display: flex; justify-content: space-between; align-items: center;
  padding: 1.5rem 1.5rem 0;
}
.modal-header h2 { font: var(--md-sys-typescale-title-large); color: var(--md-sys-color-on-surface); }
.modal-close {
  background: none; border: none; font-size: 1.5rem;
  cursor: pointer; color: var(--md-sys-color-on-surface-variant); padding: 0 0.25rem;
}
.modal-close:hover { color: var(--md-sys-color-on-surface); }
.modal-body { padding: 1.5rem; }
@media (max-width: 600px) {
  .modal-content { width: 100%; max-height: 100vh; border-radius: 0; }
}
@media (prefers-reduced-motion: reduce) {
  .modal-content { animation: none; }
}
</style>
