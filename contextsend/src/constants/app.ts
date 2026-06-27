// 适配器 logo（按应用名小写索引；Vite 处理为打包后的资源 URL）。
import janLogo from '../assets/adapter-logos/jan.png'
import chatboxLogo from '../assets/adapter-logos/chatbox.png'

/** 导航 Tab 标识 */
export const TAB_RECEIVE = 'receive' as const
export const TAB_DEVICES = 'devices' as const
export const TAB_SETTINGS = 'settings' as const
export type TabId = typeof TAB_RECEIVE | typeof TAB_DEVICES | typeof TAB_SETTINGS
export const ALL_TABS = [TAB_DEVICES, TAB_RECEIVE, TAB_SETTINGS] as const

/** 支持的 Chat AI 适配器应用名（与 Rust 端 `builtin_adapter_names()` 对齐） */
export const ADAPTER_JAN = 'Jan' as const
export const ADAPTER_CHATBOX = 'ChatBox' as const

/** 适配器名（小写）→ logo 资源 URL。未收录的适配器在 UI 用首字母占位。 */
export const ADAPTER_LOGOS: Record<string, string> = {
  jan: janLogo,
  chatbox: chatboxLogo,
}

/** 设备名最大字符数（Unicode 码点，非字节） */
export const NAME_MAX_LENGTH = 32

/** 外部链接 */
export const GITHUB_URL = 'https://github.com/zouxiaofei1/ContextSend'
