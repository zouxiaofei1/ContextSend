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

// ---- 校验范围 ----

/** 自定义端口号合法范围 */
export const PORT_MIN = 0
export const PORT_MAX = 65535

/** 连接超时合法范围（秒） */
export const TIMEOUT_MIN = 1
export const TIMEOUT_MAX = 300
