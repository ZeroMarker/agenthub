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
import ExtensionsView from './components/ExtensionsView.vue'

const activeView = ref<'agents' | 'config' | 'skills' | 'prompts' | 'sessions' | 'memory' | 'diagnostic' | 'management' | 'extensions'>('agents')
const railExpanded = ref(false)
</script>

<template>
  <div class="app-layout">
    <a href="#main-content" class="skip-link">Skip to main content</a>

    <!-- M3 Navigation Rail -->
    <nav :class="['nav-rail', { expanded: railExpanded }]" role="navigation" aria-label="Main navigation">
      <div class="nav-rail-brand">
        <span class="brand-icon">
          <svg width="24" height="24" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
            <path d="M13 2 3 14h9l-1 8 10-12h-9l1-8z" fill="currentColor" stroke="currentColor" stroke-width="1.6" stroke-linejoin="round"/>
          </svg>
        </span>
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
            { id: 'extensions', icon: '🔌', label: 'Extensions' },
          ]"
          :key="item.id"
          :class="['nav-rail-btn', { active: activeView === item.id }]"
          @click="activeView = item.id as typeof activeView"
          @keydown.enter="activeView = item.id as typeof activeView"
          :aria-label="`${item.label} view`"
          :title="item.label"
        >
          <span class="nav-rail-icon">{{ item.icon }}</span>
          <span class="nav-rail-label">{{ item.label }}</span>
        </button>
      </div>
      <button
        class="rail-toggle"
        :aria-expanded="railExpanded"
        :aria-label="railExpanded ? 'Collapse navigation labels' : 'Expand navigation labels'"
        @click="railExpanded = !railExpanded"
      >
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
          <polyline v-if="railExpanded" points="15 6 9 12 15 18"/>
          <polyline v-else points="9 6 15 12 9 18"/>
        </svg>
        <span class="rail-toggle-label">{{ railExpanded ? 'Collapse' : 'Expand' }}</span>
      </button>
    </nav>

    <main id="main-content" :class="['main-content', { 'rail-expanded': railExpanded }]">
      <AgentList v-if="activeView === 'agents'" />
      <ConfigManager v-else-if="activeView === 'config'" />
      <SkillManager v-else-if="activeView === 'skills'" />
      <PromptManager v-else-if="activeView === 'prompts'" />
      <SessionManager v-else-if="activeView === 'sessions'" />
      <MemoryManager v-else-if="activeView === 'memory'" />
      <DiagnosticView v-else-if="activeView === 'diagnostic'" />
      <ManagementView v-else-if="activeView === 'management'" />
      <ExtensionsView v-else-if="activeView === 'extensions'" />
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

.nav-rail.expanded { width: 240px; }

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
  flex-shrink: 0;
  color: var(--md-sys-color-primary);
  display: flex;
  align-items: center;
}

.brand-label {
  font: var(--md-sys-typescale-title-medium);
  color: var(--md-sys-color-primary);
  opacity: 0;
  transition: opacity var(--md-sys-motion-duration-emphasized) var(--md-sys-motion-easing-emphasized);
}

.nav-rail.expanded .brand-label { opacity: 1; }

.nav-rail-items {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  width: 100%;
  padding: 0 0.5rem;
}

.nav-rail-btn {
  position: relative;
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 0.75rem;
  border: none;
  border-radius: var(--md-sys-shape-expressive-md);
  background: transparent;
  color: var(--md-sys-color-on-surface-variant);
  font: var(--md-sys-typescale-label-medium);
  cursor: pointer;
  width: 100%;
  overflow: hidden;
  white-space: nowrap;
  transition: background var(--md-sys-motion-duration-short) var(--md-sys-motion-easing-emphasized),
              color var(--md-sys-motion-duration-short) var(--md-sys-motion-easing-emphasized),
              transform var(--md-sys-motion-duration-spring) var(--md-sys-motion-easing-spring);
}

/* Expressive active indicator: tertiary pill on the leading edge. */
.nav-rail-btn.active::before {
  content: '';
  position: absolute;
  left: -0.25rem;
  top: 50%;
  width: 5px;
  height: 55%;
  border-radius: var(--md-sys-shape-full);
  background: var(--md-sys-color-tertiary);
  transform: translateY(-50%) scaleY(0.9);
  transition: transform var(--md-sys-motion-duration-spring) var(--md-sys-motion-easing-spring);
}

.nav-rail-btn:hover {
  background: var(--md-sys-color-secondary-container);
  color: var(--md-sys-color-on-secondary-container);
  transform: translateX(2px);
}

.nav-rail-btn:active {
  transform: translateX(0) scale(0.98);
  transition-duration: var(--md-sys-motion-duration-short);
}

.nav-rail-btn.active {
  background: var(--md-sys-color-secondary-container);
  color: var(--md-sys-color-on-secondary-container);
  box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--md-sys-color-secondary) 30%, transparent);
}

.nav-rail-btn.active:hover {
  transform: none;
}

.nav-rail-btn.active:active {
  transform: scale(0.98);
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

.nav-rail.expanded .nav-rail-label { opacity: 1; }

/* --- Rail expand/collapse toggle --- */
.rail-toggle {
  margin-top: auto;
  display: flex;
  align-items: center;
  gap: 0.75rem;
  width: calc(100% - 1rem);
  padding: 0.625rem 0.75rem;
  border: none;
  border-radius: var(--md-sys-shape-expressive-md);
  background: transparent;
  color: var(--md-sys-color-on-surface-variant);
  font: var(--md-sys-typescale-label-medium);
  cursor: pointer;
  overflow: hidden;
  white-space: nowrap;
  transition: background var(--md-sys-motion-duration-short) var(--md-sys-motion-easing-emphasized),
              color var(--md-sys-motion-duration-short) var(--md-sys-motion-easing-emphasized);
}
.rail-toggle:hover {
  background: var(--md-sys-color-secondary-container);
  color: var(--md-sys-color-on-secondary-container);
}
.rail-toggle svg { flex-shrink: 0; }
.rail-toggle-label {
  opacity: 0;
  transition: opacity var(--md-sys-motion-duration-emphasized) var(--md-sys-motion-easing-emphasized);
}
.nav-rail.expanded .rail-toggle-label { opacity: 1; }

/* --- Main content --- */
.main-content {
  margin-left: 80px;
  flex: 1;
  min-height: 100vh;
  overflow-y: auto;
  transition: margin-left var(--md-sys-motion-duration-emphasized) var(--md-sys-motion-easing-emphasized);
}
.main-content.rail-expanded { margin-left: 240px; }

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
  .brand-label,
  .main-content { transition-duration: 0s !important; }
}

/* --- Responsive: collapse rail --- */
@media (max-width: 900px) {
  .nav-rail { width: 60px; }
  .nav-rail.expanded { width: 60px; }
  .nav-rail-brand { justify-content: center; padding: 0.5rem; }
  .brand-label { display: none; }
  .nav-rail-btn { justify-content: center; padding: 0.625rem; }
  .nav-rail-label { display: none; }
  .rail-toggle { justify-content: center; width: auto; }
  .rail-toggle-label { display: none; }
  .main-content { margin-left: 60px; }
  .main-content.rail-expanded { margin-left: 60px; }
}

@media (max-width: 600px) {
  .nav-rail { width: 48px; }
  .nav-rail:hover { width: 48px; }
  .nav-rail-icon { font-size: 1rem; }
  .main-content { margin-left: 48px; }
}
</style>
