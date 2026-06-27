<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'

interface SkillInfo {
  name: string
  description: string
  version: string
  enabled: boolean
  tags: string[]
  category: string | null
  source: string
}

const skills = ref<SkillInfo[]>([])
const newName = ref('')
const newDescription = ref('')
const loading = ref(false)
const message = ref('')
const messageType = ref<'success' | 'error'>('success')

async function loadSkills() {
  loading.value = true
  try {
    skills.value = await invoke<SkillInfo[]>('list_skills')
  } catch (error) {
    showMessage(`Failed to load skills: ${error}`, 'error')
  } finally {
    loading.value = false
  }
}

async function createSkill() {
  if (!newName.value.trim()) return
  loading.value = true
  try {
    await invoke('create_skill', {
      name: newName.value.trim(),
      description: newDescription.value.trim() || 'No description',
    })
    newName.value = ''
    newDescription.value = ''
    await loadSkills()
    showMessage('Skill created', 'success')
  } catch (error) {
    showMessage(`Failed to create skill: ${error}`, 'error')
  } finally {
    loading.value = false
  }
}

async function toggleSkill(skill: SkillInfo) {
  loading.value = true
  try {
    if (skill.enabled) {
      await invoke('disable_skill', { name: skill.name })
    } else {
      await invoke('enable_skill', { name: skill.name })
    }
    await loadSkills()
    showMessage(`Skill ${skill.enabled ? 'disabled' : 'enabled'}`, 'success')
  } catch (error) {
    showMessage(`Failed to toggle skill: ${error}`, 'error')
  } finally {
    loading.value = false
  }
}

async function deleteSkill(name: string) {
  loading.value = true
  try {
    await invoke('delete_skill', { name })
    await loadSkills()
    showMessage('Skill deleted', 'success')
  } catch (error) {
    showMessage(`Failed to delete skill: ${error}`, 'error')
  } finally {
    loading.value = false
  }
}

function showMessage(msg: string, type: 'success' | 'error') {
  message.value = msg
  messageType.value = type
  setTimeout(() => message.value = '', 3000)
}

onMounted(loadSkills)
</script>

<template>
  <div class="skill-manager">
    <header class="page-header">
      <h1>Skill Manager</h1>
      <p>Manage your skills and workflows</p>
    </header>

    <div v-if="message" :class="['message', messageType]">
      {{ message }}
    </div>

    <div class="create-section">
      <h3>Create Skill</h3>
      <div class="create-form">
        <input v-model="newName" placeholder="Skill name" />
        <input v-model="newDescription" placeholder="Description" @keyup.enter="createSkill" />
        <button @click="createSkill" :disabled="loading || !newName.trim()">
          Create
        </button>
      </div>
    </div>

    <div class="skills-grid">
      <div v-for="skill in skills" :key="skill.name" :class="['skill-card', { disabled: !skill.enabled }]">
        <div class="skill-header">
          <h3>{{ skill.name }}</h3>
          <div class="badges">
            <span :class="['source-badge', skill.source]">
              {{ skill.source === 'codex' ? '📦 Codex' : '💾 Local' }}
            </span>
            <span :class="['status-badge', skill.enabled ? 'enabled' : 'disabled']">
              {{ skill.enabled ? 'Enabled' : 'Disabled' }}
            </span>
          </div>
        </div>
        <p class="skill-description">{{ skill.description }}</p>
        <div class="skill-meta">
          <span class="version">v{{ skill.version }}</span>
          <span v-if="skill.category" class="category">{{ skill.category }}</span>
        </div>
        <div v-if="skill.tags.length > 0" class="skill-tags">
          <span v-for="tag in skill.tags" :key="tag" class="tag">{{ tag }}</span>
        </div>
        <div class="skill-actions">
          <button
            :class="['toggle-btn', skill.enabled ? 'disable' : 'enable']"
            @click="toggleSkill(skill)"
            :disabled="loading"
          >
            {{ skill.enabled ? 'Disable' : 'Enable' }}
          </button>
          <button
            v-if="skill.source === 'local'"
            class="delete-btn"
            @click="deleteSkill(skill.name)"
            :disabled="loading"
          >
            Delete
          </button>
        </div>
      </div>
    </div>

    <div v-if="skills.length === 0 && !loading" class="empty">
      <p>No skills found. Create your first skill above.</p>
    </div>

    <div v-if="loading" class="loading">Loading...</div>
  </div>
</template>

<style scoped>
.skill-manager {
  padding: 2rem;
}

.page-header {
  margin-bottom: 2rem;
}

.page-header h1 {
  font-size: 1.75rem;
  color: #2c3e50;
  margin-bottom: 0.5rem;
}

.page-header p {
  color: #666;
}

.message {
  padding: 0.75rem 1rem;
  border-radius: 8px;
  margin-bottom: 1rem;
}

.message.success {
  background: #d4edda;
  color: #155724;
}

.message.error {
  background: #f8d7da;
  color: #721c24;
}

.create-section {
  background: white;
  padding: 1.5rem;
  border-radius: 12px;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.05);
  margin-bottom: 2rem;
}

.create-section h3 {
  margin-bottom: 1rem;
  color: #2c3e50;
}

.create-form {
  display: flex;
  gap: 0.75rem;
}

.create-form input {
  flex: 1;
  padding: 0.6rem 1rem;
  border: 1px solid #ddd;
  border-radius: 8px;
  font-size: 0.95rem;
}

.create-form button {
  padding: 0.6rem 1.5rem;
  background: linear-gradient(135deg, #27ae60, #2ecc71);
  color: white;
  border: none;
  border-radius: 8px;
  cursor: pointer;
  font-weight: 600;
}

.create-form button:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.skills-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
  gap: 1.5rem;
}

.skill-card {
  background: white;
  padding: 1.5rem;
  border-radius: 12px;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.05);
  transition: transform 0.2s, box-shadow 0.2s;
}

.skill-card:hover {
  transform: translateY(-2px);
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.1);
}

.skill-card.disabled {
  opacity: 0.7;
}

.skill-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: 0.75rem;
}

.skill-header h3 {
  color: #2c3e50;
  font-size: 1.1rem;
}

.badges {
  display: flex;
  gap: 0.5rem;
}

.source-badge {
  padding: 0.2rem 0.5rem;
  border-radius: 4px;
  font-size: 0.7rem;
  font-weight: 600;
}

.source-badge.codex {
  background: #e3f2fd;
  color: #1976d2;
}

.source-badge.local {
  background: #f3e5f5;
  color: #7b1fa2;
}

.status-badge {
  padding: 0.25rem 0.75rem;
  border-radius: 20px;
  font-size: 0.75rem;
  font-weight: 600;
}

.status-badge.enabled {
  background: #d4edda;
  color: #155724;
}

.status-badge.disabled {
  background: #f8d7da;
  color: #721c24;
}

.skill-description {
  color: #666;
  margin-bottom: 1rem;
  line-height: 1.5;
}

.skill-meta {
  display: flex;
  gap: 0.75rem;
  margin-bottom: 0.75rem;
}

.version {
  padding: 0.2rem 0.5rem;
  background: #e3f2fd;
  color: #1976d2;
  border-radius: 4px;
  font-size: 0.8rem;
}

.category {
  padding: 0.2rem 0.5rem;
  background: #f3e5f5;
  color: #7b1fa2;
  border-radius: 4px;
  font-size: 0.8rem;
}

.skill-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
  margin-bottom: 1rem;
}

.tag {
  padding: 0.2rem 0.5rem;
  background: #f5f5f5;
  color: #666;
  border-radius: 4px;
  font-size: 0.8rem;
}

.skill-actions {
  display: flex;
  gap: 0.75rem;
}

.toggle-btn, .delete-btn {
  flex: 1;
  padding: 0.5rem;
  border: none;
  border-radius: 6px;
  cursor: pointer;
  font-size: 0.9rem;
  font-weight: 500;
}

.toggle-btn.enable {
  background: #27ae60;
  color: white;
}

.toggle-btn.disable {
  background: #f39c12;
  color: white;
}

.delete-btn {
  background: #e74c3c;
  color: white;
}

.toggle-btn:disabled, .delete-btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.empty, .loading {
  text-align: center;
  padding: 3rem;
  color: #999;
}
</style>
