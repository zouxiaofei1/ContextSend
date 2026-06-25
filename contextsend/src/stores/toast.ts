import { defineStore } from 'pinia'
import { ref } from 'vue'
import { TOAST_MAX_VISIBLE, TOAST_DURATION } from '../constants'

/** Toast 类型，决定配色与图标。 */
export type ToastType = 'success' | 'error' | 'info' | 'warning'

/** 一条 toast 通知。 */
export interface Toast {
  id: number
  type: ToastType
  message: string
  /** 自动消失时长（毫秒）；<=0 表示不自动消失，仅手动关闭。 */
  duration: number
}

/**
 * 全局轻量通知队列。新消息追加到末尾，渲染时从上到下排列（最新在最下方）。
 * 各处只需 `toast.success(...)` / `toast.error(...)` 即可，无需关心 UI。
 */
export const useToastStore = defineStore('toast', () => {
  const toasts = ref<Toast[]>([])
  let seq = 0
  /** id -> 定时器句柄，便于手动关闭时清理。 */
  const timers = new Map<number, ReturnType<typeof setTimeout>>()

  /** 展示一条 toast，返回其 id。 */
  function show(type: ToastType, message: string, opts?: { duration?: number }): number {
    const id = ++seq
    const duration = opts?.duration ?? TOAST_DURATION[type]
    toasts.value.push({ id, type, message, duration })
    // 超出上限时挤掉最旧的一条。
    while (toasts.value.length > TOAST_MAX_VISIBLE) {
      const oldest = toasts.value.shift()
      if (oldest) clearTimer(oldest.id)
    }
    if (duration > 0) {
      timers.set(
        id,
        setTimeout(() => dismiss(id), duration),
      )
    }
    return id
  }

  function clearTimer(id: number): void {
    const handle = timers.get(id)
    if (handle !== undefined) {
      clearTimeout(handle)
      timers.delete(id)
    }
  }

  /** 关闭指定 toast。 */
  function dismiss(id: number): void {
    clearTimer(id)
    toasts.value = toasts.value.filter((t) => t.id !== id)
  }

  const success = (message: string, opts?: { duration?: number }) => show('success', message, opts)
  const error = (message: string, opts?: { duration?: number }) => show('error', message, opts)
  const info = (message: string, opts?: { duration?: number }) => show('info', message, opts)
  const warning = (message: string, opts?: { duration?: number }) => show('warning', message, opts)

  return { toasts, show, dismiss, success, error, info, warning }
})
