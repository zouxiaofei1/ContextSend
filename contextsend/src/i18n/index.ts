import { createI18n } from 'vue-i18n'
import zhCN from './zh-CN'
import enUS from './en-US'

/** 将从 settingsStore 恢复的 locale 作为初始值，默认 zh-CN。 */
export function createI18nInstance(locale = 'zh-CN') {
  return createI18n({
    legacy: false,
    locale,
    fallbackLocale: 'zh-CN',
    messages: {
      'zh-CN': zhCN,
      'en-US': enUS,
    },
  })
}

export type Locale = 'zh-CN' | 'en-US'
export const SUPPORTED_LOCALES: Locale[] = ['zh-CN', 'en-US']

/** 全局 i18n 实例引用，由 {@link registerI18n} 在 main.ts 中注入。 */
let instance: ReturnType<typeof createI18nInstance> | null = null

/** 注册全局 i18n 实例，使 {@link translate} 在组件外（store / 模块）可用。 */
export function registerI18n(i18n: ReturnType<typeof createI18nInstance>): void {
  instance = i18n
}

/**
 * 在组件外翻译文案（store、模块等非 setup 上下文）。组件内仍应使用 `useI18n()`。
 * 实例未就绪时回退返回 key 本身。
 */
export function translate(key: string, named?: Record<string, unknown>): string {
  if (!instance) return key
  const g = instance.global as unknown as {
    t: (k: string, named?: Record<string, unknown>) => string
  }
  return named ? g.t(key, named) : g.t(key)
}

export { default as zhCN } from './zh-CN'
export { default as enUS } from './en-US'
