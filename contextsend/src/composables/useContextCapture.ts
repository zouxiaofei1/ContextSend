import { onMounted, onUnmounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { useAppStore } from '../stores/app'
import { useToastStore } from '../stores/toast'

/**
 * 全局上下文捕获：在窗口**任意位置**粘贴或拖入一段文本，即自动匹配回本地会话
 * 并加入存储库（导出方向的「自动匹配正确的页面」）。
 *
 * - 焦点在输入框 / 文本域 / 可编辑元素时不拦截，保证正常粘贴/拖放不受影响。
 * - 空白文本忽略；过短片段由后端返回错误并经 toast 提示。
 *
 * 在 App 根组件挂载一次即可全局生效。
 */
export function useContextCapture(): void {
  const app = useAppStore()
  const toast = useToastStore()
  const { t } = useI18n()

  /** 目标是否为可编辑元素（此时让浏览器走默认粘贴/拖放，不拦截）。 */
  function isEditable(target: EventTarget | null): boolean {
    const el = target as HTMLElement | null
    if (!el || !el.tagName) return false
    const tag = el.tagName.toUpperCase()
    return tag === 'INPUT' || tag === 'TEXTAREA' || el.isContentEditable
  }

  /** 执行匹配并入库。 */
  async function run(raw: string): Promise<void> {
    const snippet = raw.trim()
    if (!snippet) return
    try {
      const r = await app.matchContext(snippet)
      const from = r.matched && r.app ? r.app : t('receive.localSnippet')
      app.addSegment(from, r.conversation, true)
      if (r.matched && r.app) {
        toast.success(t('receive.matchFound', { app: r.app }))
      } else {
        toast.info(t('receive.matchNone'))
      }
    } catch (e) {
      toast.error(String(e))
    }
  }

  function onPaste(e: ClipboardEvent): void {
    if (isEditable(e.target)) return
    const text = e.clipboardData?.getData('text') ?? ''
    if (text.trim()) void run(text)
  }

  function onDrop(e: DragEvent): void {
    if (isEditable(e.target)) return
    const text = e.dataTransfer?.getData('text') ?? ''
    if (!text.trim()) return
    e.preventDefault()
    void run(text)
  }

  // 允许在窗口任意位置释放拖拽（默认 dragover 会阻止 drop）。
  // 需配合 tauri.conf.json 的 `dragDropEnabled: false`，否则 webview 在原生层吞掉拖放、
  // DOM 收不到事件。总是 preventDefault 以确保 drop 触发，并避免拖入文件时 webview 跳转。
  function onDragOver(e: DragEvent): void {
    if (isEditable(e.target)) return
    e.preventDefault()
  }

  onMounted(() => {
    window.addEventListener('paste', onPaste)
    window.addEventListener('drop', onDrop)
    window.addEventListener('dragover', onDragOver)
  })

  onUnmounted(() => {
    window.removeEventListener('paste', onPaste)
    window.removeEventListener('drop', onDrop)
    window.removeEventListener('dragover', onDragOver)
  })
}
