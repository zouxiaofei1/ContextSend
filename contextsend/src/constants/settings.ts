import type { Locale } from '../i18n'

// ---- 默认值（与 loadSettings 兜底 return 保持一致） ----

export const DEFAULT_THEME = 'dark' as const
export const DEFAULT_ACCENT_COLOR = '#4C7CF3'
export const DEFAULT_LOCALE: Locale = 'zh-CN'
export const DEFAULT_MINIMIZE_TO_TRAY = true
export const DEFAULT_AUTO_START = false
export const DEFAULT_SHOW_ADVANCED = false
export const DEFAULT_ALWAYS_ON_TOP = false
export const DEFAULT_START_MINIMIZED = false
export const DEFAULT_CUSTOM_PORT = 0
export const DEFAULT_CONNECTION_TIMEOUT = 30

/**
 * 「显示/隐藏主窗口」全局快捷键，空字符串表示未设置（功能关闭）。
 * 字符串为 Tauri accelerator 语法（如 `CmdOrCtrl+Shift+C`）。
 */
export const DEFAULT_GLOBAL_SHORTCUT = ''

// ---- 校验范围 ----

/** 自定义端口号合法范围 */
export const PORT_MIN = 0
export const PORT_MAX = 65535

/** 连接超时合法范围（秒） */
export const TIMEOUT_MIN = 1
export const TIMEOUT_MAX = 300

// ---- 对话保存期限 ----

export const RETENTION_OPTIONS = [
  { value: '6h', labelKey: 'settings.advanced.retentionOptions.6h' },
  { value: '1d', labelKey: 'settings.advanced.retentionOptions.1d' },
  { value: '7d', labelKey: 'settings.advanced.retentionOptions.7d' },
  { value: '30d', labelKey: 'settings.advanced.retentionOptions.30d' },
  { value: 'unlimited', labelKey: 'settings.advanced.retentionOptions.unlimited' },
] as const

export type RetentionValue = (typeof RETENTION_OPTIONS)[number]['value']

export const DEFAULT_CONVERSATION_RETENTION: RetentionValue = 'unlimited'
export const DEFAULT_MAX_CONVERSATION_COUNT = -1

/** 最大缓存对话条数合法范围（-1 表示不限） */
export const MAX_CONVERSATION_COUNT_MIN = -1
export const MAX_CONVERSATION_COUNT_MAX = 9999

/** 将保存期限值转换为毫秒；'unlimited' 返回 null。 */
export function retentionToMs(value: RetentionValue): number | null {
  switch (value) {
    case '6h':
      return 6 * 60 * 60 * 1000
    case '1d':
      return 24 * 60 * 60 * 1000
    case '7d':
      return 7 * 24 * 60 * 60 * 1000
    case '30d':
      return 30 * 24 * 60 * 60 * 1000
    default:
      return null
  }
}
