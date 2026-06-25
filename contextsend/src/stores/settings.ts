import { defineStore } from 'pinia'
import { ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { isEnabled, enable, disable } from '@tauri-apps/plugin-autostart'
import type { Locale } from '../i18n'

/** 预设主题色，包括 hex 值和对应的 hover 变体。 */
export const ACCENT_COLORS: { hex: string; hover: string; name: string }[] = [
  { hex: '#4C7CF3', hover: '#3A5FD9', name: '经典蓝 Classic Blue' },
  { hex: '#6366F1', hover: '#4F46E5', name: '靛蓝紫 Indigo' },
  { hex: '#8B5CF6', hover: '#7C3AED', name: '紫罗兰 Violet' },
  { hex: '#EC4899', hover: '#DB2777', name: '玫红 Rose' },
  { hex: '#EF4444', hover: '#DC2626', name: '赤红 Red' },
  { hex: '#F97316', hover: '#EA580C', name: '活力橙 Orange' },
  { hex: '#D97706', hover: '#B45309', name: '琥珀金 Amber' },
  { hex: '#10B981', hover: '#059669', name: '翠绿 Emerald' },
  { hex: '#14B8A6', hover: '#0D9488', name: '青碧 Teal' },
  { hex: '#06B6D4', hover: '#0891B2', name: '天蓝 Cyan' },
]

const STORAGE_KEY = 'contextsend_settings'

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
}

function loadSettings(): SettingsData {
  try {
    const raw = localStorage.getItem(STORAGE_KEY)
    if (raw) {
      const parsed = JSON.parse(raw) as Partial<SettingsData>
      return {
        theme: parsed.theme === 'light' ? 'light' : 'dark',
        accentColor: parsed.accentColor || '#4C7CF3',
        locale: parsed.locale === 'en-US' ? 'en-US' : 'zh-CN',
        minimizeToTray: parsed.minimizeToTray !== false,
        autoStart: parsed.autoStart === true,
        showAdvanced: parsed.showAdvanced === true,
        alwaysOnTop: parsed.alwaysOnTop === true,
        startMinimized: parsed.startMinimized === true,
        customPort: typeof parsed.customPort === 'number' ? parsed.customPort : 0,
        connectionTimeout:
          typeof parsed.connectionTimeout === 'number' ? parsed.connectionTimeout : 30,
      }
    }
  } catch {
    // ignore corrupt data
  }
  return {
    theme: 'dark',
    accentColor: '#4C7CF3',
    locale: 'zh-CN',
    minimizeToTray: true,
    autoStart: false,
    showAdvanced: false,
    alwaysOnTop: false,
    startMinimized: false,
    customPort: 0,
    connectionTimeout: 30,
  }
}

function persist(data: SettingsData): void {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(data))
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
      await invoke('set_minimize_to_tray', { enabled: minimizeToTray.value })
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
    if (!Number.isFinite(port) || port < 0) {
      customPort.value = 0
    } else if (port > 65535) {
      customPort.value = 65535
    } else {
      customPort.value = Math.round(port)
    }
    invoke('set_network_port', { port: customPort.value }).catch(() => {})
  }

  function setConnectionTimeout(sec: number): void {
    if (!Number.isFinite(sec) || sec < 1) {
      connectionTimeout.value = 1
    } else if (sec > 300) {
      connectionTimeout.value = 300
    } else {
      connectionTimeout.value = Math.round(sec)
    }
    invoke('set_connection_timeout', { timeoutSecs: connectionTimeout.value }).catch(() => {})
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
    ],
    ([t, a, l, m, s, adv, atop, sm, port, timeout]) => {
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
      })
      applyTheme()
    },
    { immediate: false },
  )

  // minimizeToTray 变化时同步后端
  watch(minimizeToTray, () => {
    invoke('set_minimize_to_tray', { enabled: minimizeToTray.value }).catch(() => {})
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
    syncBackend,
  }
})
