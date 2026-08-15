<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import PageHeader from './common/PageHeader.vue'
import NotificationBar from './common/NotificationBar.vue'
import LoadingSpinner from './common/LoadingSpinner.vue'
import EmptyState from './common/EmptyState.vue'

interface MarketplaceSkill {
  name: string
  description: string
  version: string
  author: string | null
  tags: string[]
  category: string | null
  available: boolean
  installs: number
  rating_avg: number | null
  rating_count: number
}

interface MarketplaceStats {
  package_count: number
  total_installs: number
  rated_count: number
  top_rated: MarketplaceSkill[]
}

interface PluginInfo {
  manifest: {
    name: string
    version: string
    description: string | null
    author: string | null
    entry: string | null
    hooks: { event: string; command: string; description: string | null }[]
  }
  enabled: boolean
  plugin_dir: string
}

interface NotifyChannel {
  id: string
  kind: string
  enabled: boolean
  created_at: string
  url?: string
  to?: string
  from?: string
  path?: string
  subject_prefix?: string | null
}

interface ChannelResult {
  channel: string
  kind: string
  ok: boolean
  message: string
}

interface PluginRunResult {
  plugin: string
  event: string
  ok: boolean
  output: string
  duration_ms: number
}

interface UserInfo {
  id: string
  name: string
  email: string | null
  roles: string[]
}

interface PermissionInfo {
  user_id: string
  action: string
  module: string | null
  agent: string | null
}

const tab = ref<'market' | 'plugins' | 'notify' | 'users'>('market')

// ---- marketplace ----
const marketSkills = ref<MarketplaceSkill[]>([])
const marketStats = ref<MarketplaceStats | null>(null)
const marketQuery = ref('')
const marketLoading = ref(false)
const rateValue = ref(5)

// ---- plugins ----
const plugins = ref<PluginInfo[]>([])
const pluginLoading = ref(false)
const pluginName = ref('')
const pluginDir = ref('')
const hookEvent = ref('on_install')
const hookResults = ref<PluginRunResult[]>([])

// ---- notify ----
const channels = ref<NotifyChannel[]>([])
const notifyLoading = ref(false)
const newChannelId = ref('')
const newChannelKind = ref('file')
const newChannelTarget = ref('')
const newChannelFrom = ref('')
const newChannelPrefix = ref('')

const message = ref('')
const messageType = ref<'success' | 'error'>('success')

async function loadMarket() {
  marketLoading.value = true
  try {
    marketStats.value = await invoke<MarketplaceStats>('market_stats')
    marketSkills.value = marketQuery.value.trim()
      ? await invoke<MarketplaceSkill[]>('market_search', { query: marketQuery.value.trim() })
      : marketStats.value.top_rated
  } catch (error) {
    showMessage(`Failed to load marketplace: ${error}`, 'error')
  } finally {
    marketLoading.value = false
  }
}

async function marketRefresh() {
  marketLoading.value = true
  try {
    marketStats.value = await invoke<MarketplaceStats>('market_refresh')
    await loadMarket()
    showMessage('Marketplace index refreshed', 'success')
  } catch (error) {
    showMessage(`Failed to refresh: ${error}`, 'error')
  } finally {
    marketLoading.value = false
  }
}

async function marketInstall(name: string) {
  marketLoading.value = true
  try {
    await invoke('market_install', { name })
    await loadMarket()
    showMessage(`Installed ${name}`, 'success')
  } catch (error) {
    showMessage(`Failed to install: ${error}`, 'error')
  } finally {
    marketLoading.value = false
  }
}

async function marketRate(name: string) {
  const rating = Math.min(5, Math.max(1, Math.round(rateValue.value || 5)))
  marketLoading.value = true
  try {
    await invoke('market_rate', { name, rating, rater: 'gui' })
    await loadMarket()
    showMessage(`Rated ${name} ${rating}★`, 'success')
  } catch (error) {
    showMessage(`Failed to rate: ${error}`, 'error')
  } finally {
    marketLoading.value = false
  }
}

// ---- plugins ----
async function loadPlugins() {
  pluginLoading.value = true
  try {
    plugins.value = await invoke<PluginInfo[]>('list_plugins')
  } catch (error) {
    showMessage(`Failed to load plugins: ${error}`, 'error')
  } finally {
    pluginLoading.value = false
  }
}

async function registerPlugin() {
  if (!pluginName.value.trim() || !pluginDir.value.trim()) return
  pluginLoading.value = true
  try {
    await invoke('register_plugin', { name: pluginName.value.trim(), dir: pluginDir.value.trim() })
    pluginName.value = ''
    pluginDir.value = ''
    await loadPlugins()
    showMessage('Plugin registered', 'success')
  } catch (error) {
    showMessage(`Failed to register plugin: ${error}`, 'error')
  } finally {
    pluginLoading.value = false
  }
}

async function togglePlugin(plugin: PluginInfo) {
  pluginLoading.value = true
  try {
    await invoke('set_plugin_enabled', { name: plugin.manifest.name, enabled: !plugin.enabled })
    await loadPlugins()
    showMessage(`Plugin ${plugin.enabled ? 'disabled' : 'enabled'}`, 'success')
  } catch (error) {
    showMessage(`Failed to toggle plugin: ${error}`, 'error')
  } finally {
    pluginLoading.value = false
  }
}

async function unregisterPlugin(name: string) {
  if (!confirm(`Unregister plugin '${name}'? Its hooks will no longer run.`)) return
  pluginLoading.value = true
  try {
    await invoke('unregister_plugin', { name })
    await loadPlugins()
    showMessage('Plugin unregistered', 'success')
  } catch (error) {
    showMessage(`Failed to unregister plugin: ${error}`, 'error')
  } finally {
    pluginLoading.value = false
  }
}

async function runHook() {
  pluginLoading.value = true
  hookResults.value = []
  try {
    hookResults.value = await invoke<PluginRunResult[]>('run_plugin_hook', { event: hookEvent.value })
    showMessage(`Hook '${hookEvent.value}' finished`, 'success')
  } catch (error) {
    showMessage(`Failed to run hook: ${error}`, 'error')
  } finally {
    pluginLoading.value = false
  }
}

// ---- notify ----
async function loadChannels() {
  notifyLoading.value = true
  try {
    channels.value = await invoke<NotifyChannel[]>('list_notify_channels')
  } catch (error) {
    showMessage(`Failed to load channels: ${error}`, 'error')
  } finally {
    notifyLoading.value = false
  }
}

function channelTarget(channel: NotifyChannel): string {
  if (channel.url) return channel.url
  if (channel.to) return `${channel.to} (from ${channel.from ?? '-'})`
  if (channel.path) return channel.path
  return '-'
}

async function addChannel() {
  if (!newChannelId.value.trim() || !newChannelTarget.value.trim()) return
  notifyLoading.value = true
  try {
    await invoke('add_notify_channel', {
      id: newChannelId.value.trim(),
      kind: newChannelKind.value,
      target: newChannelTarget.value.trim(),
      from: newChannelKind.value === 'email' ? newChannelFrom.value.trim() || null : null,
      subjectPrefix: newChannelKind.value === 'email' ? newChannelPrefix.value.trim() || null : null,
    })
    newChannelId.value = ''
    newChannelTarget.value = ''
    newChannelFrom.value = ''
    newChannelPrefix.value = ''
    await loadChannels()
    showMessage('Channel added', 'success')
  } catch (error) {
    showMessage(`Failed to add channel: ${error}`, 'error')
  } finally {
    notifyLoading.value = false
  }
}

async function toggleChannel(channel: NotifyChannel) {
  notifyLoading.value = true
  try {
    await invoke('set_notify_channel_enabled', { id: channel.id, enabled: !channel.enabled })
    await loadChannels()
    showMessage(`Channel ${channel.enabled ? 'disabled' : 'enabled'}`, 'success')
  } catch (error) {
    showMessage(`Failed to toggle channel: ${error}`, 'error')
  } finally {
    notifyLoading.value = false
  }
}

async function removeChannel(id: string) {
  if (!confirm(`Remove notification channel '${id}'?`)) return
  notifyLoading.value = true
  try {
    await invoke('remove_notify_channel', { id })
    await loadChannels()
    showMessage('Channel removed', 'success')
  } catch (error) {
    showMessage(`Failed to remove channel: ${error}`, 'error')
  } finally {
    notifyLoading.value = false
  }
}

async function sendTest() {
  notifyLoading.value = true
  try {
    const results = await invoke<ChannelResult[]>('send_notification')
    if (results.length === 0) {
      showMessage('No enabled channels — add one first', 'error')
    } else {
      const failed = results.filter((r) => !r.ok)
      showMessage(
        failed.length === 0
          ? `Alert delivered to ${results.length} channel(s)`
          : `${failed.length} channel(s) failed`,
        failed.length === 0 ? 'success' : 'error'
      )
      await loadChannels()
    }
  } catch (error) {
    showMessage(`Failed to send: ${error}`, 'error')
  } finally {
    notifyLoading.value = false
  }
}

// ---- users & permissions ----
const users = ref<UserInfo[]>([])
const permissions = ref<PermissionInfo[]>([])
const usersLoading = ref(false)
const newUserId = ref('')
const newUserName = ref('')
const newUserEmail = ref('')
const newUserRoles = ref('viewer')
const permUser = ref('')
const permAction = ref('read')
const permModule = ref('')
const permAgent = ref('')

async function loadUsers() {
  usersLoading.value = true
  try {
    users.value = await invoke<UserInfo[]>('list_users')
    permissions.value = await invoke<PermissionInfo[]>('list_permissions', { user: null })
  } catch (error) {
    showMessage(`Failed to load users: ${error}`, 'error')
  } finally {
    usersLoading.value = false
  }
}

async function createUser() {
  if (!newUserId.value.trim() || !newUserName.value.trim()) return
  usersLoading.value = true
  try {
    await invoke('create_user', {
      id: newUserId.value.trim(),
      name: newUserName.value.trim(),
      email: newUserEmail.value.trim() || null,
      roles: newUserRoles.value.split(',').map((r) => r.trim()).filter(Boolean),
    })
    newUserId.value = ''
    newUserName.value = ''
    newUserEmail.value = ''
    await loadUsers()
    showMessage('User created', 'success')
  } catch (error) {
    showMessage(`Failed to create user: ${error}`, 'error')
  } finally {
    usersLoading.value = false
  }
}

async function deleteUser(id: string) {
  if (!confirm(`Delete user '${id}'? Their permissions will be removed too.`)) return
  usersLoading.value = true
  try {
    await invoke('delete_user', { id })
    await loadUsers()
    showMessage(`User '${id}' deleted`, 'success')
  } catch (error) {
    showMessage(`Failed to delete user: ${error}`, 'error')
  } finally {
    usersLoading.value = false
  }
}

async function toggleRole(user: UserInfo, role: string) {
  usersLoading.value = true
  try {
    const hasRole = user.roles.includes(role)
    if (hasRole) {
      await invoke('remove_user_role', { id: user.id, role })
    } else {
      await invoke('add_user_role', { id: user.id, role })
    }
    await loadUsers()
    showMessage(`Role '${role}' ${hasRole ? 'removed from' : 'added to'} ${user.id}`, 'success')
  } catch (error) {
    showMessage(`Failed to update role: ${error}`, 'error')
  } finally {
    usersLoading.value = false
  }
}

async function grantPermission() {
  if (!permUser.value.trim()) return
  usersLoading.value = true
  try {
    await invoke('grant_permission', {
      user: permUser.value.trim(),
      action: permAction.value,
      module: permModule.value.trim() || null,
      agent: permAgent.value.trim() || null,
    })
    await loadUsers()
    showMessage('Permission granted', 'success')
  } catch (error) {
    showMessage(`Failed to grant permission: ${error}`, 'error')
  } finally {
    usersLoading.value = false
  }
}

async function revokePermission(permission: PermissionInfo) {
  usersLoading.value = true
  try {
    await invoke('revoke_permission', {
      user: permission.user_id,
      action: permission.action,
      module: permission.module,
      agent: permission.agent,
    })
    await loadUsers()
    showMessage('Permission revoked', 'success')
  } catch (error) {
    showMessage(`Failed to revoke permission: ${error}`, 'error')
  } finally {
    usersLoading.value = false
  }
}

function switchTab(next: typeof tab.value) {
  tab.value = next
  if (next === 'plugins') loadPlugins()
  if (next === 'notify') loadChannels()
  if (next === 'market') loadMarket()
  if (next === 'users') loadUsers()
}

function showMessage(msg: string, type: 'success' | 'error') {
  message.value = msg
  messageType.value = type
  setTimeout(() => (message.value = ''), 3500)
}

onMounted(loadMarket)
</script>

<template>
  <div class="extensions-view">
    <PageHeader title="Extensions" subtitle="Skill marketplace · plugins · alert notifications" />

    <NotificationBar :message="message" :type="messageType" @close="message = ''" />

    <div class="m3-tabs" role="tablist" aria-label="Extensions sections">
      <button
        v-for="t in [
          { id: 'market', label: 'Marketplace' },
          { id: 'plugins', label: 'Plugins' },
          { id: 'notify', label: 'Notifications' },
          { id: 'users', label: 'Users' },
        ]"
        :key="t.id"
        :class="['m3-tab', { active: tab === t.id }]"
        role="tab"
        :aria-selected="tab === t.id"
        @click="switchTab(t.id as typeof tab)"
      >
        {{ t.label }}
      </button>
    </div>

    <!-- ============ Marketplace ============ -->
    <section v-if="tab === 'market'" class="panel">
      <div class="toolbar">
        <input
          v-model="marketQuery"
          class="search-box"
          placeholder="Search by name, description or tag…"
          @keyup.enter="loadMarket"
        />
        <button class="m3-btn-tonal" @click="loadMarket">Search</button>
        <button class="m3-btn-outlined" @click="marketRefresh" :disabled="marketLoading">Refresh index</button>
      </div>

      <div v-if="marketStats" class="stat-chips">
        <span class="stat-chip">{{ marketStats.package_count }} packages</span>
        <span class="stat-chip">{{ marketStats.total_installs }} installs</span>
        <span class="stat-chip">{{ marketStats.rated_count }} rated</span>
      </div>

      <LoadingSpinner v-if="marketLoading" />
      <EmptyState v-else-if="marketSkills.length === 0" text="No marketplace packages. Add one with `agenthub skill market add-package`." />

      <div v-else class="market-list">
        <div v-for="skill in marketSkills" :key="skill.name" class="m3-card market-item">
          <div class="market-info">
            <span class="market-name">{{ skill.name }}</span>
            <span class="market-version">v{{ skill.version }}</span>
            <span class="market-desc">{{ skill.description }}</span>
            <div class="tags" v-if="skill.tags.length">
              <span v-for="tag in skill.tags" :key="tag" class="tag">{{ tag }}</span>
            </div>
          </div>
          <div class="market-meta">
            <span class="stat-chip">★ {{ skill.rating_avg?.toFixed(1) ?? '-' }} ({{ skill.rating_count }})</span>
            <span class="stat-chip">{{ skill.installs }} installs</span>
          </div>
          <div class="market-actions">
            <div class="rate-row">
              <input v-model.number="rateValue" type="number" min="1" max="5" class="rate-input" aria-label="Rating 1-5" />
              <button class="m3-btn-tonal" @click="marketRate(skill.name)" :disabled="marketLoading">Rate</button>
            </div>
            <button class="m3-btn-filled" @click="marketInstall(skill.name)" :disabled="marketLoading">
              Install
            </button>
          </div>
        </div>
      </div>
    </section>

    <!-- ============ Plugins ============ -->
    <section v-if="tab === 'plugins'" class="panel">
      <div class="register-form m3-card">
        <h3>Register plugin</h3>
        <div class="form-row">
          <input v-model="pluginName" placeholder="Plugin name" />
          <input v-model="pluginDir" placeholder="Path to directory with plugin.yaml" class="grow" />
          <button class="m3-btn-filled" @click="registerPlugin" :disabled="pluginLoading || !pluginName.trim() || !pluginDir.trim()">
            Register
          </button>
        </div>
      </div>

      <div class="hook-row">
        <label for="hook-event">Run hook event</label>
        <select id="hook-event" v-model="hookEvent">
          <option>on_install</option>
          <option>on_uninstall</option>
          <option>on_session_end</option>
          <option>on_monitor</option>
          <option>on_backup</option>
        </select>
        <button class="m3-btn-tonal" @click="runHook" :disabled="pluginLoading">Run</button>
      </div>

      <div v-if="hookResults.length" class="hook-results">
        <div
          v-for="r in hookResults"
          :key="`${r.plugin}.${r.event}`"
          :class="['hook-result', r.ok ? 'ok' : 'fail']"
        >
          <span class="hook-mark">{{ r.ok ? '✅' : '❌' }}</span>
          <span class="hook-name">{{ r.plugin }}.{{ r.event }}</span>
          <span class="hook-msg">{{ r.output || '(no output)' }} ({{ r.duration_ms }}ms)</span>
        </div>
      </div>

      <LoadingSpinner v-if="pluginLoading && plugins.length === 0" />
      <EmptyState v-else-if="plugins.length === 0" text="No plugins registered." />

      <div v-else class="plugin-list">
        <div v-for="plugin in plugins" :key="plugin.manifest.name" class="m3-card plugin-item">
          <div class="plugin-info">
            <span class="plugin-name">{{ plugin.manifest.name }}</span>
            <span class="plugin-version">v{{ plugin.manifest.version }}</span>
            <span class="plugin-desc">{{ plugin.manifest.description || 'No description' }}</span>
            <span v-if="plugin.enabled" class="status-badge ok">enabled</span>
            <span v-else class="status-badge muted">disabled</span>
          </div>
          <div class="plugin-hooks" v-if="plugin.manifest.hooks.length">
            <span v-for="h in plugin.manifest.hooks" :key="h.event" class="tag">{{ h.event }}</span>
          </div>
          <div class="plugin-actions">
            <button class="m3-btn-tonal" @click="togglePlugin(plugin)" :disabled="pluginLoading">
              {{ plugin.enabled ? 'Disable' : 'Enable' }}
            </button>
            <button class="m3-btn-outlined danger" @click="unregisterPlugin(plugin.manifest.name)" :disabled="pluginLoading">
              Unregister
            </button>
          </div>
        </div>
      </div>
    </section>

    <!-- ============ Notifications ============ -->
    <section v-if="tab === 'notify'" class="panel">
      <div class="add-form m3-card">
        <h3>Add channel</h3>
        <div class="form-grid">
          <input v-model="newChannelId" placeholder="Channel id (e.g. ops)" />
          <select v-model="newChannelKind">
            <option value="file">file</option>
            <option value="webhook">webhook</option>
            <option value="email">email</option>
          </select>
          <input
            v-model="newChannelTarget"
            :placeholder="newChannelKind === 'webhook' ? 'https://…' : newChannelKind === 'email' ? 'recipient@example.com' : 'alerts.log'"
            class="span-2"
          />
          <template v-if="newChannelKind === 'email'">
            <input v-model="newChannelFrom" placeholder="From address" />
            <input v-model="newChannelPrefix" placeholder="Subject prefix (optional)" />
          </template>
        </div>
        <button class="m3-btn-filled" @click="addChannel" :disabled="notifyLoading || !newChannelId.trim() || !newChannelTarget.trim()">
          Add
        </button>
      </div>

      <div class="toolbar">
        <button class="m3-btn-tonal" @click="sendTest" :disabled="notifyLoading">Send test alert</button>
      </div>

      <LoadingSpinner v-if="notifyLoading && channels.length === 0" />
      <EmptyState v-else-if="channels.length === 0" text="No notification channels configured." />

      <div v-else class="channel-list">
        <div v-for="channel in channels" :key="channel.id" class="m3-card channel-item">
          <div class="channel-info">
            <span class="channel-name">{{ channel.id }}</span>
            <span class="tag">{{ channel.kind }}</span>
            <span class="channel-target">{{ channelTarget(channel) }}</span>
            <span v-if="channel.enabled" class="status-badge ok">enabled</span>
            <span v-else class="status-badge muted">disabled</span>
          </div>
          <div class="channel-actions">
            <button class="m3-btn-tonal" @click="toggleChannel(channel)" :disabled="notifyLoading">
              {{ channel.enabled ? 'Disable' : 'Enable' }}
            </button>
            <button class="m3-btn-outlined danger" @click="removeChannel(channel.id)" :disabled="notifyLoading">
              Remove
            </button>
          </div>
        </div>
      </div>
    </section>

    <!-- ============ Users & Permissions ============ -->
    <section v-if="tab === 'users'" class="panel">
      <div class="register-form m3-card">
        <h3>Create user</h3>
        <div class="form-grid">
          <input v-model="newUserId" placeholder="User id" />
          <input v-model="newUserName" placeholder="Display name" />
          <input v-model="newUserEmail" placeholder="Email (optional)" />
          <input v-model="newUserRoles" placeholder="Roles, comma-separated (admin, operator, viewer)" />
        </div>
        <button class="m3-btn-filled" @click="createUser" :disabled="usersLoading || !newUserId.trim() || !newUserName.trim()">
          Create
        </button>
      </div>

      <div class="register-form m3-card">
        <h3>Grant permission</h3>
        <div class="form-grid">
          <input v-model="permUser" placeholder="User id" />
          <select v-model="permAction">
            <option>read</option>
            <option>write</option>
            <option>admin</option>
          </select>
          <input v-model="permModule" placeholder="Module (optional, e.g. config/session/…)" />
          <input v-model="permAgent" placeholder="Agent (optional)" />
        </div>
        <button class="m3-btn-filled" @click="grantPermission" :disabled="usersLoading || !permUser.trim()">
          Grant
        </button>
      </div>

      <LoadingSpinner v-if="usersLoading && users.length === 0" />
      <EmptyState v-else-if="users.length === 0" text="No users." />

      <div v-else class="user-list">
        <div v-for="user in users" :key="user.id" class="m3-card user-item">
          <div class="user-info">
            <span class="user-name">{{ user.name }}</span>
            <span class="user-id">{{ user.id }}</span>
            <span class="user-email">{{ user.email ?? '-' }}</span>
            <div class="role-row">
              <button
                v-for="role in ['admin', 'operator', 'viewer']"
                :key="role"
                :class="['role-chip', { active: user.roles.includes(role) }]"
                @click="toggleRole(user, role)"
                :disabled="usersLoading"
              >
                {{ role }}
              </button>
            </div>
          </div>
          <div class="user-perms" v-if="permissions.filter((p) => p.user_id === user.id).length">
            <span
              v-for="p in permissions.filter((x) => x.user_id === user.id)"
              :key="`${p.user_id}.${p.action}.${p.module}.${p.agent}`"
              class="tag"
            >
              {{ p.action }}:{{ p.module ?? '*' }}{{ p.agent ? `@${p.agent}` : '' }}
              <button class="perm-remove" @click="revokePermission(p)" :disabled="usersLoading" title="Revoke">✕</button>
            </span>
          </div>
          <div class="user-actions" v-if="user.id !== 'admin'">
            <button class="m3-btn-outlined danger" @click="deleteUser(user.id)" :disabled="usersLoading">
              Delete
            </button>
          </div>
        </div>
      </div>
    </section>
  </div>
</template>

<style scoped>
.extensions-view { padding: 2rem; }

.panel { display: flex; flex-direction: column; gap: 1rem; }
.toolbar { display: flex; gap: 0.75rem; align-items: center; }
.search-box { flex: 1; max-width: 360px; padding: 0.6rem 1rem; border: 1px solid var(--md-sys-color-outline-variant); border-radius: var(--md-sys-shape-sm); background: var(--md-sys-color-surface); color: var(--md-sys-color-on-surface); }
.stat-chips { display: flex; gap: 0.5rem; flex-wrap: wrap; }

.m3-btn-outlined.danger { color: var(--md-sys-color-error); border-color: var(--md-sys-color-error); }

.market-item { display: flex; flex-direction: column; gap: 0.75rem; }
.market-info { display: flex; flex-direction: column; gap: 0.25rem; }
.market-name { font: var(--md-sys-typescale-title-medium); color: var(--md-sys-color-on-surface); }
.market-version, .market-desc { color: var(--md-sys-color-on-surface-variant); font-size: 0.9rem; }
.market-meta, .market-actions { display: flex; gap: 0.5rem; align-items: center; flex-wrap: wrap; }
.rate-row { display: flex; gap: 0.4rem; align-items: center; }
.rate-input { width: 56px; padding: 0.4rem; border: 1px solid var(--md-sys-color-outline-variant); border-radius: var(--md-sys-shape-xs); background: var(--md-sys-color-surface); color: var(--md-sys-color-on-surface); }

.register-form h3, .add-form h3 { margin-bottom: 0.75rem; color: var(--md-sys-color-on-surface); font: var(--md-sys-typescale-title-medium); }
.form-row { display: flex; gap: 0.75rem; align-items: center; }
.form-row input, .add-form input, .add-form select { padding: 0.6rem 1rem; border: 1px solid var(--md-sys-color-outline-variant); border-radius: var(--md-sys-shape-sm); background: var(--md-sys-color-surface); color: var(--md-sys-color-on-surface); }
.form-row .grow { flex: 1; }
.form-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 0.75rem; margin-bottom: 0.75rem; }
.span-2 { grid-column: span 2; }

.hook-row { display: flex; gap: 0.75rem; align-items: center; }
.hook-row select { padding: 0.5rem; border: 1px solid var(--md-sys-color-outline-variant); border-radius: var(--md-sys-shape-xs); background: var(--md-sys-color-surface); color: var(--md-sys-color-on-surface); }
.hook-results { display: flex; flex-direction: column; gap: 0.4rem; }
.hook-result { display: flex; gap: 0.6rem; align-items: center; padding: 0.5rem 0.75rem; border-radius: var(--md-sys-shape-sm); font-size: 0.9rem; }
.hook-result.ok { background: var(--md-sys-color-secondary-container); color: var(--md-sys-color-on-secondary-container); }
.hook-result.fail { background: var(--md-sys-color-error-container); color: var(--md-sys-color-on-error-container); }
.hook-name { font-weight: 600; }
.hook-msg { color: inherit; opacity: 0.8; }

.plugin-item, .channel-item { display: flex; flex-direction: column; gap: 0.75rem; }
.plugin-info, .channel-info { display: flex; align-items: center; gap: 0.6rem; flex-wrap: wrap; }
.plugin-name, .channel-name { font: var(--md-sys-typescale-title-medium); color: var(--md-sys-color-on-surface); }
.plugin-version, .plugin-desc, .channel-target { color: var(--md-sys-color-on-surface-variant); font-size: 0.9rem; }
.plugin-hooks { display: flex; gap: 0.4rem; flex-wrap: wrap; }
.plugin-actions, .channel-actions { display: flex; gap: 0.6rem; }
.status-badge { padding: 0.15rem 0.6rem; border-radius: var(--md-sys-shape-full); font-size: 0.75rem; font-weight: 600; }
.status-badge.ok { background: var(--md-sys-color-tertiary-container); color: var(--md-sys-color-on-tertiary-container); }
.status-badge.muted { background: var(--md-sys-color-surface-variant); color: var(--md-sys-color-on-surface-variant); }

/* Users */
.user-item { display: flex; flex-direction: column; gap: 0.75rem; }
.user-info { display: flex; align-items: center; gap: 0.6rem; flex-wrap: wrap; }
.user-name { font: var(--md-sys-typescale-title-medium); color: var(--md-sys-color-on-surface); }
.user-id, .user-email { color: var(--md-sys-color-on-surface-variant); font-size: 0.9rem; }
.role-row { display: flex; gap: 0.4rem; }
.role-chip { padding: 0.25rem 0.7rem; border-radius: var(--md-sys-shape-full); border: 1px solid var(--md-sys-color-outline); background: transparent; color: var(--md-sys-color-on-surface-variant); font-size: 0.75rem; cursor: pointer; }
.role-chip.active { background: var(--md-sys-color-secondary-container); border-color: transparent; color: var(--md-sys-color-on-secondary-container); }
.user-perms { display: flex; gap: 0.4rem; flex-wrap: wrap; }
.perm-remove { margin-left: 0.25rem; border: none; background: transparent; color: inherit; cursor: pointer; opacity: 0.7; }
.perm-remove:hover { opacity: 1; }
.user-actions { display: flex; gap: 0.6rem; }

@media (max-width: 700px) {
  .extensions-view { padding: 1.25rem; }
  .form-row, .toolbar { flex-direction: column; align-items: stretch; }
  .search-box { max-width: none; }
  .form-grid { grid-template-columns: 1fr; }
  .span-2 { grid-column: span 1; }
}
</style>
