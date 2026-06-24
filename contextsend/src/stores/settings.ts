import { defineStore } from 'pinia'
import { ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { enable, disable, isEnabled } from '@tauri-apps/plugin-autostart'
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

  /** 将 CSS 变量和 data-theme 应用至 DOM。 */
  function applyTheme(): void {
    document.documentElement.setAttribute('data-theme', theme.value)
    document.documentElement.style.setProperty('--accent', accentColor.value)
    const entry = ACCENT_COLORS.find((c) => c.hex === accentColor.value)
    document.documentElement.style.setProperty(
      '--accent-hover',
      entry?.hover || accentColor.value,
    )
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

  /** 持久化所有设置到 localStorage，并同步 DOM 主题 + 后端。 */
  watch(
    [theme, accentColor, locale, minimizeToTray, autoStart],
    ([t, a, l, m, s]) => {
      persist({ theme: t, accentColor: a, locale: l, minimizeToTray: m, autoStart: s })
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
    applyTheme,
    toggleTheme,
    setAccentColor,
    setLocale,
    toggleMinimizeToTray,
    toggleAutoStart,
    syncBackend,
  }
})
