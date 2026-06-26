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

export interface ThemeVariant {
  accent: string
  accentHover: string
}
export interface Theme {
  id: string
  /** 显示名（英语） */
  name: string
  /** 英文显示名（与 name 保持一致） */
  nameEn: string
  dark: ThemeVariant
  light: ThemeVariant
}

export const THEMES: Theme[] = [
  {
    id: 'midnight',
    name: 'Midnight',
    nameEn: 'Midnight',
    dark: { accent: '#6E8BFA', accentHover: '#566FE0' },
    light: { accent: '#3F5BD9', accentHover: '#3147B8' },
  },
  {
    id: 'dawn',
    name: 'Dawn',
    nameEn: 'Dawn',
    dark: { accent: '#F472B6', accentHover: '#E85AA0' },
    light: { accent: '#DB2777', accentHover: '#BE1D63' },
  },
  {
    id: 'dusk',
    name: 'Dusk',
    nameEn: 'Dusk',
    dark: { accent: '#FB923C', accentHover: '#F47A20' },
    light: { accent: '#EA580C', accentHover: '#C2480A' },
  },
]

/** 各模式背景的中性基色（与 styles.css 的回退值保持一致）。 */
const NEUTRALS = {
  dark: { primary: '#121317', secondary: '#181a1f', tertiary: '#1e2127', border: '#262930' },
  light: { primary: '#ebebeb', secondary: '#f5f5f5', tertiary: '#dedede', border: '#cacaca' },
} as const

/** 强调色混入背景的比例（加深着色，让背景带有更强的主题倾向）。 */
const TINT = {
  dark: { primary: 0.03, secondary: 0.05, tertiary: 0.10, border: 0.15 },
  light: { primary: 0.06, secondary: 0.06, tertiary: 0.10, border: 0.15 },
} as const

function hexToRgb(hex: string): [number, number, number] {
  const h = hex.replace('#', '')
  return [
    parseInt(h.slice(0, 2), 16),
    parseInt(h.slice(2, 4), 16),
    parseInt(h.slice(4, 6), 16),
  ]
}

function rgbToHex(r: number, g: number, b: number): string {
  const c = (n: number) => Math.round(Math.max(0, Math.min(255, n))).toString(16).padStart(2, '0')
  return `#${c(r)}${c(g)}${c(b)}`
}

/** 将 accent 按比例 t 混入 base，返回 hex。 */
function mix(base: string, accent: string, t: number): string {
  const [br, bg, bb] = hexToRgb(base)
  const [ar, ag, ab] = hexToRgb(accent)
  return rgbToHex(br + (ar - br) * t, bg + (ag - bg) * t, bb + (ab - bb) * t)
}

/** 由主题与模式派生出全部主题相关 CSS 变量。 */
export function buildThemeVars(theme: Theme, mode: 'light' | 'dark'): Record<string, string> {
  const v = theme[mode]
  const n = NEUTRALS[mode]
  const t = TINT[mode]
  return {
    '--accent': v.accent,
    '--accent-hover': v.accentHover,
    '--bg-primary': mix(n.primary, v.accent, t.primary),
    '--bg-secondary': mix(n.secondary, v.accent, t.secondary),
    '--bg-tertiary': mix(n.tertiary, v.accent, t.tertiary),
    '--border': mix(n.border, v.accent, t.border),
  }
}
