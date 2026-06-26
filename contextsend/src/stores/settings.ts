import { defineStore } from 'pinia'
import { ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { isEnabled, enable, disable } from '@tauri-apps/plugin-autostart'
import type { Locale } from '../i18n'
import {
  ACCENT_COLORS,
  LS_SETTINGS,
  DEFAULT_THEME,
  DEFAULT_ACCENT_COLOR,
  DEFAULT_LOCALE,
  DEFAULT_MINIMIZE_TO_TRAY,
  DEFAULT_AUTO_START,
  DEFAULT_SHOW_ADVANCED,
  DEFAULT_ALWAYS_ON_TOP,
  DEFAULT_START_MINIMIZED,
  DEFAULT_CUSTOM_PORT,
  DEFAULT_CONNECTION_TIMEOUT,
  DEFAULT_CONVERSATION_RETENTION,
  DEFAULT_MAX_CONVERSATION_COUNT,
  PORT_MIN,
  PORT_MAX,
  TIMEOUT_MIN,
  TIMEOUT_MAX,
  MAX_CONVERSATION_COUNT_MIN,
  MAX_CONVERSATION_COUNT_MAX,
  IPC,
} from '../constants'
import type { RetentionValue } from '../constants'


interface SettingsData {
  theme: 'light' | 'dark'
  accentColor: string
  locale: Locale
  minimizeToTray: boolean
  autoStart: boolean
  showAdvanced: boolean
  alwaysOnTop: boolean
  startMinimized: boolean
  customPort: number
  connectionTimeout: number
  conversationRetention: RetentionValue
  maxConversationCount: number
}

function loadSettings(): SettingsData {
  try {
    const raw = localStorage.getItem(LS_SETTINGS)
    if (raw) {
      const parsed = JSON.parse(raw) as Partial<SettingsData>
      return {
        theme: parsed.theme === 'light' ? 'light' : DEFAULT_THEME,
        accentColor: parsed.accentColor || DEFAULT_ACCENT_COLOR,
        locale: parsed.locale === 'en-US' ? 'en-US' : DEFAULT_LOCALE,
        minimizeToTray: parsed.minimizeToTray !== false,
        autoStart: parsed.autoStart === true,
        showAdvanced: parsed.showAdvanced === true,
        alwaysOnTop: parsed.alwaysOnTop === true,
        startMinimized: parsed.startMinimized === true,
        customPort: typeof parsed.customPort === 'number' ? parsed.customPort : DEFAULT_CUSTOM_PORT,
        connectionTimeout:
          typeof parsed.connectionTimeout === 'number'
            ? parsed.connectionTimeout
            : DEFAULT_CONNECTION_TIMEOUT,
        conversationRetention: isRetentionValue(parsed.conversationRetention)
          ? parsed.conversationRetention
          : DEFAULT_CONVERSATION_RETENTION,
        maxConversationCount:
          typeof parsed.maxConversationCount === 'number'
            ? parsed.maxConversationCount
            : DEFAULT_MAX_CONVERSATION_COUNT,
      }
    }
  } catch {
    // ignore corrupt data
  }
  return {
    theme: DEFAULT_THEME,
    accentColor: DEFAULT_ACCENT_COLOR,
    locale: DEFAULT_LOCALE,
    minimizeToTray: DEFAULT_MINIMIZE_TO_TRAY,
    autoStart: DEFAULT_AUTO_START,
    showAdvanced: DEFAULT_SHOW_ADVANCED,
    alwaysOnTop: DEFAULT_ALWAYS_ON_TOP,
    startMinimized: DEFAULT_START_MINIMIZED,
    customPort: DEFAULT_CUSTOM_PORT,
    connectionTimeout: DEFAULT_CONNECTION_TIMEOUT,
    conversationRetention: DEFAULT_CONVERSATION_RETENTION,
    maxConversationCount: DEFAULT_MAX_CONVERSATION_COUNT,
  }
}

function isRetentionValue(v: unknown): v is RetentionValue {
  return v === '6h' || v === '1d' || v === '7d' || v === '30d' || v === 'unlimited'
}

function persist(data: SettingsData): void {
  localStorage.setItem(LS_SETTINGS, JSON.stringify(data))
}

export const useSettingsStore = defineStore('settings', () => {
  const initial = loadSettings()

  const theme = ref<'light' | 'dark'>(initial.theme)
  const accentColor = ref<string>(initial.accentColor)
  const locale = ref<Locale>(initial.locale)
  const minimizeToTray = ref<boolean>(initial.minimizeToTray)
  const autoStart = ref<boolean>(initial.autoStart)
  const showAdvanced = ref<boolean>(initial.showAdvanced)
  const alwaysOnTop = ref<boolean>(initial.alwaysOnTop)
  const startMinimized = ref<boolean>(initial.startMinimized)
  const customPort = ref<number>(initial.customPort)
  const connectionTimeout = ref<number>(initial.connectionTimeout)
  const conversationRetention = ref<RetentionValue>(initial.conversationRetention)
  const maxConversationCount = ref<number>(initial.maxConversationCount)

  /** 将 CSS 变量和 data-theme 应用至 DOM。 */
  function applyTheme(): void {
    document.documentElement.setAttribute('data-theme', theme.value)
    document.documentElement.style.setProperty('--accent', accentColor.value)
    const entry = ACCENT_COLORS.find((c) => c.hex === accentColor.value)
    document.documentElement.style.setProperty('--accent-hover', entry?.hover || accentColor.value)
  }

  /** 将 minimizeToTray 和 autoStart 同步到 Rust 后端。 */
  async function syncBackend(): Promise<void> {
    try {
      await invoke(IPC.SET_MINIMIZE_TO_TRAY, { enabled: minimizeToTray.value })
    } catch {
      // 后端可能未就绪
    }
    try {
      const currentlyEnabled = await isEnabled()
      if (autoStart.value && !currentlyEnabled) {
        await enable()
      } else if (!autoStart.value && currentlyEnabled) {
        await disable()
      }
    } catch {
      // autostart plugin 可能不可用
    }
  }

  function toggleTheme(): void {
    theme.value = theme.value === 'dark' ? 'light' : 'dark'
  }

  function setAccentColor(hex: string): void {
    const entry = ACCENT_COLORS.find((c) => c.hex === hex)
    if (!entry) return
    accentColor.value = hex
  }

  function setLocale(loc: Locale): void {
    locale.value = loc
  }

  function toggleMinimizeToTray(): void {
    minimizeToTray.value = !minimizeToTray.value
  }

  async function toggleAutoStart(): Promise<void> {
    autoStart.value = !autoStart.value
    await syncBackend()
  }

  function toggleShowAdvanced(): void {
    showAdvanced.value = !showAdvanced.value
  }

  function toggleAlwaysOnTop(): void {
    alwaysOnTop.value = !alwaysOnTop.value
    getCurrentWindow()
      .setAlwaysOnTop(alwaysOnTop.value)
      .catch(() => {})
  }

  function toggleStartMinimized(): void {
    startMinimized.value = !startMinimized.value
  }

  function setCustomPort(port: number): void {
    if (!Number.isFinite(port) || port < PORT_MIN) {
      customPort.value = PORT_MIN
    } else if (port > PORT_MAX) {
      customPort.value = PORT_MAX
    } else {
      customPort.value = Math.round(port)
    }
    invoke(IPC.SET_NETWORK_PORT, { port: customPort.value }).catch(() => {})
  }

  function setConnectionTimeout(sec: number): void {
    if (!Number.isFinite(sec) || sec < TIMEOUT_MIN) {
      connectionTimeout.value = TIMEOUT_MIN
    } else if (sec > TIMEOUT_MAX) {
      connectionTimeout.value = TIMEOUT_MAX
    } else {
      connectionTimeout.value = Math.round(sec)
    }
    invoke(IPC.SET_CONNECTION_TIMEOUT, { timeoutSecs: connectionTimeout.value }).catch(() => {})
  }

  function setConversationRetention(value: RetentionValue): void {
    conversationRetention.value = value
  }

  function setMaxConversationCount(value: number): void {
    const n = Number.isFinite(value) ? Math.round(value) : DEFAULT_MAX_CONVERSATION_COUNT
    if (n < MAX_CONVERSATION_COUNT_MIN) {
      maxConversationCount.value = MAX_CONVERSATION_COUNT_MIN
    } else if (n > MAX_CONVERSATION_COUNT_MAX) {
      maxConversationCount.value = MAX_CONVERSATION_COUNT_MAX
    } else {
      maxConversationCount.value = n
    }
  }

  /** 持久化所有设置到 localStorage，并同步 DOM 主题 + 后端。 */
  watch(
    [
      theme,
      accentColor,
      locale,
      minimizeToTray,
      autoStart,
      showAdvanced,
      alwaysOnTop,
      startMinimized,
      customPort,
      connectionTimeout,
      conversationRetention,
      maxConversationCount,
    ],
    ([t, a, l, m, s, adv, atop, sm, port, timeout, retention, maxCount]) => {
      persist({
        theme: t,
        accentColor: a,
        locale: l,
        minimizeToTray: m,
        autoStart: s,
        showAdvanced: adv,
        alwaysOnTop: atop,
        startMinimized: sm,
        customPort: port,
        connectionTimeout: timeout,
        conversationRetention: retention,
        maxConversationCount: maxCount,
      })
      applyTheme()
    },
    { immediate: false },
  )

  // minimizeToTray 变化时同步后端
  watch(minimizeToTray, () => {
    invoke(IPC.SET_MINIMIZE_TO_TRAY, { enabled: minimizeToTray.value }).catch(() => {})
  })

  return {
    theme,
    accentColor,
    locale,
    minimizeToTray,
    autoStart,
    showAdvanced,
    alwaysOnTop,
    startMinimized,
    customPort,
    connectionTimeout,
    conversationRetention,
    maxConversationCount,
    applyTheme,
    toggleTheme,
    setAccentColor,
    setLocale,
    toggleMinimizeToTray,
    toggleAutoStart,
    toggleShowAdvanced,
    toggleAlwaysOnTop,
    toggleStartMinimized,
    setCustomPort,
    setConnectionTimeout,
    setConversationRetention,
    setMaxConversationCount,
    syncBackend,
  }
})
