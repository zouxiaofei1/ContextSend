import type { ToastType } from '../stores/toast'

// ---- 布局 ----

/** 竖屏切换阈值：窗口宽度低于此值时切换为底部 Tab 布局 */
export const PORTRAIT_BREAKPOINT = 500
/** 侧边栏折叠阈值：宽度低于此值时侧边栏仅显示图标 */
export const COMPACT_BREAKPOINT = 720

/** 关键组件尺寸（px），需与对应组件 CSS 保持同步 */
export const TITLEBAR_HEIGHT = 36
export const SIDEBAR_WIDTH = 220

// ---- Toast ----

/** Toast 同时可见最大条数 */
export const TOAST_MAX_VISIBLE = 4

/** Toast 各类型默认持续时间（ms） */
export const TOAST_DURATION: Record<ToastType, number> = {
  success: 3000,
  info: 3000,
  warning: 4000,
  error: 5000,
}

// ---- 主题 ----

/** 预设主题色板 */
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
