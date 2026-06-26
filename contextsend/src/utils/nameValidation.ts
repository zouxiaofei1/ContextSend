import { NAME_MAX_LENGTH } from '../constants/app'

export interface NameValidation {
  valid: boolean
  error: 'tooLong' | 'empty' | null
}

/**
 * 校验设备名：非空、不超过 NAME_MAX_LENGTH 个字符。
 * 注意 `.length` 对 BMP 外字符不准，这里用 `[...]` 按 Unicode 码点计数。
 */
export function validateName(name: string): NameValidation {
  const trimmed = name.trim()
  if (trimmed.length === 0) return { valid: false, error: 'empty' }
  if ([...trimmed].length > NAME_MAX_LENGTH) return { valid: false, error: 'tooLong' }
  return { valid: true, error: null }
}

/**
 * 用于显示：超过限长则截断并追加 "…"（U+2026）。
 * 默认使用 NAME_MAX_LENGTH；调用方可传入更小的显示宽度。
 */
export function displayName(name: string, maxLen = NAME_MAX_LENGTH): string {
  const chars = [...name]
  return chars.length > maxLen ? chars.slice(0, maxLen).join('') + '…' : name
}
