import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import AgentList from '../AgentList.vue'
import AgentCard from '../agent/AgentCard.vue'
import EmptyState from '../common/EmptyState.vue'
import LoadingSpinner from '../common/LoadingSpinner.vue'
import NotificationBar from '../common/NotificationBar.vue'

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }))

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}))

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
  type: {},
}))

interface FixtureAgent {
  id: string
  name: string
  description: string
  kind: 'CLI' | 'Desktop'
  provider: string
  homepage: string
  status: string
  installers: { platform: string; manager: string; package: string | null }[]
  catalog_verified_at: string | null
  installer_verified_at: string | null
}

const fixtureAgents: FixtureAgent[] = [
  {
    id: 'claude-code',
    name: 'Claude Code',
    description: 'Anthropic terminal agent',
    kind: 'CLI',
    provider: 'Anthropic',
    homepage: 'https://anthropic.com/claude-code',
    status: 'verified',
    installers: [{ platform: 'windows', manager: 'npm', package: '@anthropic-ai/claude-code' }],
    catalog_verified_at: '2026-01-01',
    installer_verified_at: '2026-01-01',
  },
  {
    id: 'codex',
    name: 'Codex',
    description: 'OpenAI CLI coding agent',
    kind: 'CLI',
    provider: 'OpenAI',
    homepage: 'https://openai.com/codex',
    status: 'verified',
    installers: [{ platform: 'windows', manager: 'npm', package: '@openai/codex' }],
    catalog_verified_at: null,
    installer_verified_at: null,
  },
  {
    id: 'cursor',
    name: 'Cursor',
    description: 'AI-powered code editor',
    kind: 'Desktop',
    provider: 'Cursor',
    homepage: 'https://cursor.com',
    status: 'verified',
    installers: [{ platform: 'windows', manager: 'winget', package: 'Anysphere.Cursor' }],
    catalog_verified_at: null,
    installer_verified_at: null,
  },
]

const mockList = (agents: FixtureAgent[] = fixtureAgents) => {
  invokeMock.mockImplementation((cmd: string) => {
    switch (cmd) {
      case 'list_agents':
        return Promise.resolve(agents)
      case 'list_installed_agents':
        return Promise.resolve([])
      case 'search_agents':
        return Promise.resolve(agents)
      default:
        return Promise.resolve({})
    }
  })
}

function mountList() {
  const wrapper = mount(AgentList, { attachTo: document.body })
  return wrapper
}

beforeEach(() => {
  vi.clearAllMocks()
  localStorage.clear()
  mockList()
})

afterEach(() => {
  vi.useRealTimers()
  document.body.innerHTML = ''
})

describe('AgentList', () => {
  it('loads and renders all agents from invoke with stats', async () => {
    const wrapper = mountList()
    await flushPromises()

    const cards = wrapper.findAllComponents(AgentCard)
    expect(cards).toHaveLength(3)
    expect(invokeMock).toHaveBeenCalledWith('list_agents', { agentType: null })

    const stats = wrapper.findAll('.stat-chip')
    const statText = stats.map((s) => s.text())
    expect(statText.some((t) => t.includes('3') && t.includes('Total'))).toBe(true)
    expect(statText.some((t) => t.includes('2') && t.includes('CLI'))).toBe(true)
    expect(statText.some((t) => t.includes('1') && t.includes('Desktop'))).toBe(true)
  })

  it('renders empty state when no agents returned', async () => {
    mockList([])
    const wrapper = mountList()
    await flushPromises()

    expect(wrapper.findComponent(EmptyState).exists()).toBe(true)
    expect(wrapper.text()).toContain('No agents found')
  })

  it('shows loading spinner during initial load', async () => {
    let resolveList: (v: FixtureAgent[]) => void = () => {}
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'list_agents') return new Promise((r) => (resolveList = r))
      if (cmd === 'list_installed_agents') return Promise.resolve([])
      return Promise.resolve({})
    })

    const wrapper = mountList()
    await wrapper.vm.$nextTick()
    expect(wrapper.findComponent(LoadingSpinner).exists()).toBe(true)
    resolveList(fixtureAgents)
    await flushPromises()
    expect(wrapper.findComponent(LoadingSpinner).exists()).toBe(false)
  })

  it('filters agents by CLI tab', async () => {
    const wrapper = mountList()
    await flushPromises()

    const cliTab = wrapper.findAll('.m3-tab').find((b) => b.text().includes('CLI Agents'))
    expect(cliTab).toBeDefined()
    await cliTab!.trigger('click')
    await flushPromises()

    const cards = wrapper.findAllComponents(AgentCard)
    expect(cards).toHaveLength(2)
    for (const card of cards) {
      expect(card.text()).not.toContain('Cursor')
    }
  })

  it('filters agents by Desktop tab', async () => {
    const wrapper = mountList()
    await flushPromises()

    const desktopTab = wrapper.findAll('.m3-tab').find((b) => b.text().includes('Desktop Agents'))
    await desktopTab!.trigger('click')
    await flushPromises()

    const cards = wrapper.findAllComponents(AgentCard)
    expect(cards).toHaveLength(1)
    expect(cards[0].text()).toContain('Cursor')
  })

  it('debounced search filters agents by name', async () => {
    vi.useFakeTimers()
    const wrapper = mountList()
    await vi.advanceTimersByTimeAsync(0)
    await flushPromises()

    const input = wrapper.find('.search-bar input')
    await input.setValue('codex')
    await vi.advanceTimersByTimeAsync(300)
    await flushPromises()

    const cards = wrapper.findAllComponents(AgentCard)
    expect(cards).toHaveLength(1)
    expect(cards[0].text()).toContain('Codex')
  })

  it('sorts agents by name descending when toggled', async () => {
    const wrapper = mountList()
    await flushPromises()

    const nameChip = wrapper.findAll('.m3-chip').find((b) => b.text().includes('Name'))
    await nameChip!.trigger('click')
    await flushPromises()

    const titles = wrapper.findAllComponents(AgentCard).map((c) => c.find('.card-title').text())
    expect(titles).toEqual(['Cursor', 'Codex', 'Claude Code'])
  })

  it('selects all visible agents and runs batch install', async () => {
    const wrapper = mountList()
    await flushPromises()

    await wrapper.find('.batch-select input').setValue(true)
    const installBtn = wrapper.findAll('.batch-btns button').find((b) => b.text().includes('Install'))
    expect(installBtn!.attributes('disabled')).toBeUndefined()

    await installBtn!.trigger('click')
    await flushPromises()

    expect(invokeMock).toHaveBeenCalledWith(
      'batch_install_agents',
      expect.objectContaining({ names: expect.any(Array) }),
    )
    const batchCall = invokeMock.mock.calls.find((c) => c[0] === 'batch_install_agents')
    expect(batchCall![1].names.sort()).toEqual(['claude-code', 'codex', 'cursor'].sort())
  })

  it('shows error notification when install fails', async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'list_agents') return Promise.resolve(fixtureAgents)
      if (cmd === 'list_installed_agents') return Promise.resolve([])
      if (cmd === 'install_agent') return Promise.reject(new Error('simulated failure'))
      return Promise.resolve({})
    })

    const wrapper = mountList()
    await flushPromises()

    const installBtn = wrapper.findAllComponents(AgentCard)[0].find('button')
    await installBtn.trigger('click')
    await flushPromises()

    const notif = wrapper.findComponent(NotificationBar)
    expect(notif.exists()).toBe(true)
    expect(notif.text()).toContain('simulated failure')
  })

  it('shows cancel button during install and invokes cancel_operation', async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'list_agents') return Promise.resolve(fixtureAgents)
      if (cmd === 'list_installed_agents') return Promise.resolve([])
      if (cmd === 'install_agent') return new Promise(() => {}) // never resolves
      return Promise.resolve({})
    })

    const wrapper = mountList()
    await flushPromises()

    await wrapper.findAllComponents(AgentCard)[0].find('button').trigger('click')
    await flushPromises()

    const card = wrapper.findAllComponents(AgentCard)[0]
    const cancelBtn = card.findAll('button').find((b) => b.text().includes('Cancel'))
    expect(cancelBtn).toBeDefined()
    await cancelBtn!.trigger('click')
    await flushPromises()

    expect(invokeMock).toHaveBeenCalledWith('cancel_operation', { name: fixtureAgents[0].id })
  })

  it('shows retry button and failure details after a failed install result', async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'list_agents') return Promise.resolve(fixtureAgents)
      if (cmd === 'list_installed_agents') return Promise.resolve([])
      if (cmd === 'install_agent') {
        return Promise.resolve({
          success: false,
          message: 'Install failed',
          agent_name: fixtureAgents[0].name,
          command: 'npm install -g @anthropic-ai/claude-code',
          exit_code: 1,
          stdout: '',
          stderr: 'EACCES: permission denied',
          duration_ms: 500,
          timed_out: false,
        })
      }
      return Promise.resolve({})
    })

    const wrapper = mountList()
    await flushPromises()

    await wrapper.findAllComponents(AgentCard)[0].find('button').trigger('click')
    await flushPromises()

    const card = wrapper.findAllComponents(AgentCard)[0]
    expect(card.text()).toContain('Failed')
    expect(card.text()).toContain('Install failed')
    const retryBtn = card.findAll('button').find((b) => b.text().includes('Retry'))
    expect(retryBtn).toBeDefined()

    // Expand failure details
    const details = card.find('details')
    expect(details.exists()).toBe(true)
    await details.find('summary').trigger('click')
    expect(card.text()).toContain('EACCES: permission denied')
    expect(card.text()).toContain('npm install -g @anthropic-ai/claude-code')
  })
})
