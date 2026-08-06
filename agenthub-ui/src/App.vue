<script setup lang="ts">
import { ref } from 'vue'
import AgentList from './components/AgentList.vue'
import ConfigManager from './components/ConfigManager.vue'
import SkillManager from './components/SkillManager.vue'
import PromptManager from './components/PromptManager.vue'
import SessionManager from './components/SessionManager.vue'
import MemoryManager from './components/MemoryManager.vue'
import DiagnosticView from './components/DiagnosticView.vue'
import ManagementView from './components/ManagementView.vue'

const activeView = ref<'agents' | 'config' | 'skills' | 'prompts' | 'sessions' | 'memory' | 'diagnostic' | 'management'>('agents')
</script>

<template>
  <div class="app-layout">
    <a href="#main-content" class="skip-link">Skip to main content</a>

    <!-- M3 Navigation Rail -->
    <nav class="nav-rail" role="navigation" aria-label="Main navigation">
      <div class="nav-rail-brand">
        <span class="brand-icon">⚡</span>
        <span class="brand-label">AgentHub</span>
      </div>
      <div class="nav-rail-items">
        <button
          v-for="item in [
            { id: 'agents', icon: '📦', label: 'Agents' },
            { id: 'config', icon: '⚙️', label: 'Config' },
            { id: 'skills', icon: '🛠️', label: 'Skills' },
            { id: 'prompts', icon: '📝', label: 'Prompts' },
            { id: 'sessions', icon: '💬', label: 'Sessions' },
            { id: 'memory', icon: '🧠', label: 'Memory' },
            { id: 'diagnostic', icon: '🩺', label: 'Diagnostic' },
            { id: 'management', icon: '📊', label: 'Overview' },
          ]"
          :key="item.id"
          :class="['nav-rail-btn', { active: activeView === item.id }]"
          @click="activeView = item.id as typeof activeView"
          @keydown.enter="activeView = item.id as typeof activeView"
          :aria-label="`${item.label} view`"
        >
          <span class="nav-rail-icon">{{ item.icon }}</span>
          <span class="nav-rail-label">{{ item.label }}</span>
        </button>
      </div>
    </nav>

    <main id="main-content" class="main-content">
      <AgentList v-if="activeView === 'agents'" />
      <ConfigManager v-else-if="activeView === 'config'" />
      <SkillManager v-else-if="activeView === 'skills'" />
      <PromptManager v-else-if="activeView === 'prompts'" />
      <SessionManager v-else-if="activeView === 'sessions'" />
      <MemoryManager v-else-if="activeView === 'memory'" />
      <DiagnosticView v-else-if="activeView === 'diagnostic'" />
      <ManagementView v-else-if="activeView === 'management'" />
    </main>
  </div>
</template>

<style>
/* ============================================================
   App Layout — M3 Navigation Rail
   ============================================================ */

* { margin: 0; padding: 0; box-sizing: border-box; }

.app-layout {
  display: flex;
  min-height: 100vh;
  background: var(--md-sys-color-background);
}

/* --- Skip link --- */
.skip-link {
  position: absolute;
  top: -40px;
  left: 0;
  background: var(--md-sys-color-primary);
  color: var(--md-sys-color-on-primary);
  padding: 0.5rem 1rem;
  z-index: 9999;
  transition: top var(--md-sys-motion-duration-short) var(--md-sys-motion-easing-emphasized);
  text-decoration: none;
  font: var(--md-sys-typescale-label-large);
}
.skip-link:focus { top: 0; }

/* --- Navigation Rail --- */
.nav-rail {
  position: fixed;
  top: 0;
  left: 0;
  width: 80px;
  height: 100vh;
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 0.75rem 0 1rem;
  background: var(--md-sys-color-surface);
  border-right: 1px solid var(--md-sys-color-outline-variant);
  z-index: 100;
  overflow-y: auto;
  transition: width var(--md-sys-motion-duration-emphasized) var(--md-sys-motion-easing-emphasized);
}

.nav-rail:hover { width: 240px; }

.nav-rail-brand {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 0.75rem 1rem;
  margin-bottom: 0.5rem;
  width: 100%;
  overflow: hidden;
  white-space: nowrap;
}

.brand-icon {
  font-size: 1.5rem;
  flex-shrink: 0;
}

.brand-label {
  font: var(--md-sys-typescale-title-medium);
  color: var(--md-sys-color-primary);
  opacity: 0;
  transition: opacity var(--md-sys-motion-duration-emphasized) var(--md-sys-motion-easing-emphasized);
}

.nav-rail:hover .brand-label { opacity: 1; }

.nav-rail-items {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  width: 100%;
  padding: 0 0.5rem;
}

.nav-rail-btn {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 0.75rem;
  border: none;
  border-radius: var(--md-sys-shape-xl);
  background: transparent;
  color: var(--md-sys-color-on-surface-variant);
  font: var(--md-sys-typescale-label-medium);
  cursor: pointer;
  width: 100%;
  overflow: hidden;
  white-space: nowrap;
  transition: background var(--md-sys-motion-duration-short) var(--md-sys-motion-easing-emphasized),
              color var(--md-sys-motion-duration-short) var(--md-sys-motion-easing-emphasized);
}

.nav-rail-btn:hover {
  background: var(--md-sys-color-secondary-container);
  color: var(--md-sys-color-on-secondary-container);
}

.nav-rail-btn.active {
  background: var(--md-sys-color-secondary-container);
  color: var(--md-sys-color-on-secondary-container);
}

.nav-rail-icon {
  font-size: 1.25rem;
  flex-shrink: 0;
  width: 1.5rem;
  text-align: center;
}

.nav-rail-label {
  font: var(--md-sys-typescale-label-large);
  opacity: 0;
  transition: opacity var(--md-sys-motion-duration-emphasized) var(--md-sys-motion-easing-emphasized);
}

.nav-rail:hover .nav-rail-label { opacity: 1; }

/* --- Main content --- */
.main-content {
  margin-left: 80px;
  flex: 1;
  min-height: 100vh;
  overflow-y: auto;
}

/* --- Reduced motion --- */
@media (prefers-reduced-motion: reduce) {
  *, *::before, *::after {
    animation-duration: 0.01ms !important;
    animation-iteration-count: 1 !important;
    transition-duration: 0.01ms !important;
    scroll-behavior: auto !important;
  }
  .nav-rail,
  .nav-rail-label,
  .brand-label { transition-duration: 0s !important; }
  .nav-rail:hover { width: 80px; }
  .nav-rail:hover .nav-rail-label,
  .nav-rail:hover .brand-label { opacity: 0; }
}

/* --- Responsive: collapse rail --- */
@media (max-width: 900px) {
  .nav-rail { width: 60px; }
  .nav-rail:hover { width: 60px; }
  .nav-rail-brand { justify-content: center; padding: 0.5rem; }
  .brand-label { display: none; }
  .nav-rail-btn { justify-content: center; padding: 0.625rem; }
  .nav-rail-label { display: none; }
  .main-content { margin-left: 60px; }
}

@media (max-width: 600px) {
  .nav-rail { width: 48px; }
  .nav-rail:hover { width: 48px; }
  .nav-rail-icon { font-size: 1rem; }
  .main-content { margin-left: 48px; }
}
</style>
