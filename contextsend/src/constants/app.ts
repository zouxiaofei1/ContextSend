/** 导航 Tab 标识 */
export const TAB_RECEIVE = 'receive' as const
export const TAB_DEVICES = 'devices' as const
export const TAB_SETTINGS = 'settings' as const
export type TabId = typeof TAB_RECEIVE | typeof TAB_DEVICES | typeof TAB_SETTINGS
export const ALL_TABS = [TAB_RECEIVE, TAB_DEVICES, TAB_SETTINGS] as const

/** 支持的 Chat AI 适配器应用名（与 Rust 端 `builtin_adapter_names()` 对齐） */
export const ADAPTER_JAN = 'Jan' as const
export const ADAPTER_CHATBOX = 'ChatBox' as const

/** 外部链接 */
export const GITHUB_URL = 'https://github.com/zouxiaofei1/ContextSend'
