import { defineStore } from 'pinia'
import { ref, computed, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { isEnabled, enable, disable } from '@tauri-apps/plugin-autostart'
import type { Locale, LangPreference } from '../i18n'
import { detectSystemLocale } from '../i18n'
import {
  THEMES,
  buildThemeVars,
  LS_SETTINGS,
  DEFAULT_THEME_ID,
  LEGACY_ACCENT_TO_THEME,
  DEFAULT_LANG_PREFERENCE,
  DEFAULT_MINIMIZE_TO_TRAY,
  DEFAULT_AUTO_START,
  DEFAULT_SHOW_ADVANCED,
  DEFAULT_ALWAYS_ON_TOP,
  DEFAULT_START_MINIMIZED,
  DEFAULT_CUSTOM_PORT,
  DEFAULT_CONNECTION_TIMEOUT,
  DEFAULT_GLOBAL_SHORTCUT,
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
  themeId: string
  langPreference: LangPreference
  minimizeToTray: boolean
  autoStart: boolean
  showAdvanced: boolean
  alwaysOnTop: boolean
  startMinimized: boolean
  customPort: number
  connectionTimeout: number
  globalShortcut: string
  conversationRetention: RetentionValue
  maxConversationCount: number
}

function loadSettings(): SettingsData {
  try {
    const raw = localStorage.getItem(LS_SETTINGS)
    if (raw) {
      const parsed = JSON.parse(raw) as Partial<SettingsData> & {
        accentColor?: string
        /** 旧版字段：直接持久化的实际 locale，迁移为 langPreference。 */
        locale?: string
      }
      return {
        themeId: resolveThemeId(parsed.themeId, parsed.accentColor),
        langPreference: resolveLangPreference(parsed),
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
        globalShortcut:
          typeof parsed.globalShortcut === 'string'
            ? parsed.globalShortcut
            : DEFAULT_GLOBAL_SHORTCUT,
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
    themeId: DEFAULT_THEME_ID,
    langPreference: DEFAULT_LANG_PREFERENCE,
    minimizeToTray: DEFAULT_MINIMIZE_TO_TRAY,
    autoStart: DEFAULT_AUTO_START,
    showAdvanced: DEFAULT_SHOW_ADVANCED,
    alwaysOnTop: DEFAULT_ALWAYS_ON_TOP,
    startMinimized: DEFAULT_START_MINIMIZED,
    customPort: DEFAULT_CUSTOM_PORT,
    connectionTimeout: DEFAULT_CONNECTION_TIMEOUT,
    globalShortcut: DEFAULT_GLOBAL_SHORTCUT,
    conversationRetention: DEFAULT_CONVERSATION_RETENTION,
    maxConversationCount: DEFAULT_MAX_CONVERSATION_COUNT,
  }
}

function isRetentionValue(v: unknown): v is RetentionValue {
  return v === '6h' || v === '1d' || v === '7d' || v === '30d' || v === 'unlimited'
}

/**
 * 解析持久化的语言偏好：优先用新版 langPreference；否则迁移旧版 locale 字段
 * （已选具体语言的老用户保留其选择）；都无效时回退默认（跟随系统）。
 */
function resolveLangPreference(parsed: {
  langPreference?: LangPreference
  locale?: string
}): LangPreference {
  const pref = parsed.langPreference ?? parsed.locale
  if (pref === 'system' || pref === 'zh-CN' || pref === 'en-US') return pref
  return DEFAULT_LANG_PREFERENCE
}

/**
 * 解析持久化的主题：优先用新版 themeId；否则尝试从旧版 accentColor（hex）迁移；
 * 都无效时回退默认主题。
 */
function resolveThemeId(themeId: string | undefined, legacyAccent: string | undefined): string {
  if (themeId && THEMES.some((t) => t.id === themeId)) return themeId
  if (legacyAccent && LEGACY_ACCENT_TO_THEME[legacyAccent]) {
    return LEGACY_ACCENT_TO_THEME[legacyAccent]
  }
  return DEFAULT_THEME_ID
}

function persist(data: SettingsData): void {
  localStorage.setItem(LS_SETTINGS, JSON.stringify(data))
}

export const useSettingsStore = defineStore('settings', () => {
  const initial = loadSettings()

  const themeId = ref<string>(initial.themeId)
  // 深浅模式跟随系统：不持久化，由 prefers-color-scheme 派生并实时跟随。
  const media = window.matchMedia('(prefers-color-scheme: dark)')
  const colorScheme = ref<'light' | 'dark'>(media.matches ? 'dark' : 'light')
  media.addEventListener('change', (e) => {
    colorScheme.value = e.matches ? 'dark' : 'light'
    applyTheme()
  })

  const langPreference = ref<LangPreference>(initial.langPreference)
  // 系统语言：偏好为 `system` 时实际生效的 locale，随系统语言变化实时跟随。
  const systemLocale = ref<Locale>(detectSystemLocale())
  window.addEventListener('languagechange', () => {
    systemLocale.value = detectSystemLocale()
  })
  /** 实际生效的语言：偏好为 `system` 时派生自系统语言，否则即偏好本身。 */
  const locale = computed<Locale>(() =>
    langPreference.value === 'system' ? systemLocale.value : langPreference.value,
  )
  const minimizeToTray = ref<boolean>(initial.minimizeToTray)
  const autoStart = ref<boolean>(initial.autoStart)
  const showAdvanced = ref<boolean>(initial.showAdvanced)
  const alwaysOnTop = ref<boolean>(initial.alwaysOnTop)
  const startMinimized = ref<boolean>(initial.startMinimized)
  const customPort = ref<number>(initial.customPort)
  const connectionTimeout = ref<number>(initial.connectionTimeout)
  const globalShortcut = ref<string>(initial.globalShortcut)
  const conversationRetention = ref<RetentionValue>(initial.conversationRetention)
  const maxConversationCount = ref<number>(initial.maxConversationCount)

  /** 将 CSS 变量和 data-theme 应用至 DOM。深浅由系统决定，配色由命名主题决定。 */
  function applyTheme(): void {
    document.documentElement.setAttribute('data-theme', colorScheme.value)
    const theme = THEMES.find((t) => t.id === themeId.value) ?? THEMES[0]
    const vars = buildThemeVars(theme, colorScheme.value)
    for (const [key, value] of Object.entries(vars)) {
      document.documentElement.style.setProperty(key, value)
    }
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
    // 重新注册持久化的全局快捷键（空字符串等价于不注册）。
    try {
      await invoke(IPC.SET_GLOBAL_SHORTCUT, { accelerator: globalShortcut.value || null })
    } catch {
      // 热键可能被占用或后端未就绪；启动期静默，由用户在设置里重设。
    }
  }

  function setThemeId(id: string): void {
    if (!THEMES.some((t) => t.id === id)) return
    themeId.value = id
  }

  function setLangPreference(pref: LangPreference): void {
    langPreference.value = pref
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

  /**
   * 设置或清除「显示/隐藏主窗口」全局快捷键。
   *
   * 传空字符串清除热键。先经后端校验/注册，失败（语法非法或被占用）时回滚到旧值
   * 并抛出错误信息，交由调用方提示用户。成功才更新本地状态并持久化。
   */
  async function setGlobalShortcut(accelerator: string): Promise<void> {
    const next = accelerator.trim()
    if (next === globalShortcut.value) return
    const previous = globalShortcut.value
    try {
      await invoke(IPC.SET_GLOBAL_SHORTCUT, { accelerator: next || null })
      globalShortcut.value = next
    } catch (e) {
      // 注册失败：保持旧值不变，向上抛出供 UI 提示。
      globalShortcut.value = previous
      throw e instanceof Error ? e : new Error(String(e))
    }
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
      themeId,
      langPreference,
      minimizeToTray,
      autoStart,
      showAdvanced,
      alwaysOnTop,
      startMinimized,
      customPort,
      connectionTimeout,
      globalShortcut,
      conversationRetention,
      maxConversationCount,
    ],
    ([tid, l, m, s, adv, atop, sm, port, timeout, shortcut, retention, maxCount]) => {
      persist({
        themeId: tid,
        langPreference: l,
        minimizeToTray: m,
        autoStart: s,
        showAdvanced: adv,
        alwaysOnTop: atop,
        startMinimized: sm,
        customPort: port,
        connectionTimeout: timeout,
        globalShortcut: shortcut,
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
    themeId,
    colorScheme,
    langPreference,
    locale,
    minimizeToTray,
    autoStart,
    showAdvanced,
    alwaysOnTop,
    startMinimized,
    customPort,
    connectionTimeout,
    globalShortcut,
    conversationRetention,
    maxConversationCount,
    applyTheme,
    setThemeId,
    setLangPreference,
    toggleMinimizeToTray,
    toggleAutoStart,
    toggleShowAdvanced,
    toggleAlwaysOnTop,
    toggleStartMinimized,
    setCustomPort,
    setConnectionTimeout,
    setGlobalShortcut,
    setConversationRetention,
    setMaxConversationCount,
    syncBackend,
  }
})
