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

/** 供全局引用的 i18n 实例（在 main.ts 中赋值）。 */
export { default as zhCN } from './zh-CN'
export { default as enUS } from './en-US'
