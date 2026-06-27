<script setup lang="ts">
import { ref } from 'vue'
import AgentList from './components/AgentList.vue'
import ConfigManager from './components/ConfigManager.vue'
import SkillManager from './components/SkillManager.vue'
import PromptManager from './components/PromptManager.vue'
import SessionManager from './components/SessionManager.vue'
import MemoryManager from './components/MemoryManager.vue'
import DiagnosticView from './components/DiagnosticView.vue'

const activeView = ref<'agents' | 'config' | 'skills' | 'prompts' | 'sessions' | 'memory' | 'diagnostic'>('agents')
</script>

<template>
  <div class="app-layout">
    <nav class="sidebar">
      <div class="nav-brand">
        <h2>AgentHub</h2>
      </div>
      <ul class="nav-links">
        <li>
          <button
            :class="['nav-btn', { active: activeView === 'agents' }]"
            @click="activeView = 'agents'"
          >
            <span class="nav-icon">📦</span>
            <span>Agents</span>
          </button>
        </li>
        <li>
          <button
            :class="['nav-btn', { active: activeView === 'config' }]"
            @click="activeView = 'config'"
          >
            <span class="nav-icon">⚙️</span>
            <span>Config</span>
          </button>
        </li>
        <li>
          <button
            :class="['nav-btn', { active: activeView === 'skills' }]"
            @click="activeView = 'skills'"
          >
            <span class="nav-icon">🛠️</span>
            <span>Skills</span>
          </button>
        </li>
        <li>
          <button
            :class="['nav-btn', { active: activeView === 'prompts' }]"
            @click="activeView = 'prompts'"
          >
            <span class="nav-icon">📝</span>
            <span>Prompts</span>
          </button>
        </li>
        <li>
          <button
            :class="['nav-btn', { active: activeView === 'sessions' }]"
            @click="activeView = 'sessions'"
          >
            <span class="nav-icon">💬</span>
            <span>Sessions</span>
          </button>
        </li>
        <li>
          <button
            :class="['nav-btn', { active: activeView === 'memory' }]"
            @click="activeView = 'memory'"
          >
            <span class="nav-icon">🧠</span>
            <span>Memory</span>
          </button>
        </li>
        <li>
          <button
            :class="['nav-btn', { active: activeView === 'diagnostic' }]"
            @click="activeView = 'diagnostic'"
          >
            <span class="nav-icon">🩺</span>
            <span>Diagnostic</span>
          </button>
        </li>
      </ul>
    </nav>
    <main class="main-content">
      <AgentList v-if="activeView === 'agents'" />
      <ConfigManager v-else-if="activeView === 'config'" />
      <SkillManager v-else-if="activeView === 'skills'" />
      <PromptManager v-else-if="activeView === 'prompts'" />
      <SessionManager v-else-if="activeView === 'sessions'" />
      <MemoryManager v-else-if="activeView === 'memory'" />
      <DiagnosticView v-else-if="activeView === 'diagnostic'" />
    </main>
  </div>
</template>

<style>
* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

body {
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
  background: #f5f7fa;
}

.app-layout {
  display: flex;
  min-height: 100vh;
}

.sidebar {
  width: 220px;
  background: linear-gradient(180deg, #1a1a2e 0%, #16213e 100%);
  color: white;
  padding: 1.5rem 0;
  flex-shrink: 0;
}

.nav-brand {
  padding: 0 1.5rem 1.5rem;
  border-bottom: 1px solid rgba(255, 255, 255, 0.1);
  margin-bottom: 1rem;
}

.nav-brand h2 {
  font-size: 1.5rem;
  font-weight: 700;
  background: linear-gradient(135deg, #667eea, #764ba2);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
}

.nav-links {
  list-style: none;
  padding: 0 0.75rem;
}

.nav-links li {
  margin-bottom: 0.25rem;
}

.nav-btn {
  width: 100%;
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 0.75rem 1rem;
  border: none;
  border-radius: 8px;
  background: transparent;
  color: rgba(255, 255, 255, 0.7);
  font-size: 0.95rem;
  cursor: pointer;
  transition: all 0.2s;
}

.nav-btn:hover {
  background: rgba(255, 255, 255, 0.1);
  color: white;
}

.nav-btn.active {
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  color: white;
  box-shadow: 0 2px 8px rgba(102, 126, 234, 0.3);
}

.nav-icon {
  font-size: 1.2rem;
}

.main-content {
  flex: 1;
  overflow-y: auto;
}
</style>
