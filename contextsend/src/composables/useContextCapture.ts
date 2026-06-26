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
 * - 窗口内部发起的拖拽（如选中文本后拖动）会被忽略，仅处理外部来源的拖入。
 *
 * 在 App 根组件挂载一次即可全局生效。
 */
export function useContextCapture(): void {
  const app = useAppStore()
  const toast = useToastStore()
  const { t } = useI18n()

  /**
   * 标记当前拖拽是否源自窗口内部（选中文本拖拽等）。
   * 若为 true，则 drop 时忽略，避免将内部拖拽误判为外部上下文拖入。
   * 利用 HTML5 规范：dragstart 仅在拖拽起源于同一 document 时触发。
   */
  let isInternalDrag = false

  /** 目标是否为可编辑元素（此时让浏览器走默认粘贴/拖放，不拦截）。 */
  function isEditable(target: EventTarget | null): boolean {
    const el = target as HTMLElement | null
    if (!el || !el.tagName) return false
    const tag = el.tagName.toUpperCase()
    return tag === 'INPUT' || tag === 'TEXTAREA' || el.isContentEditable
  }

  /**
   * 窗口内部发起的拖拽（如选中文本后拖动）会触发 dragstart；
   * 外部拖入（从浏览器、文件管理器等）不会在窗口内触发此事件。
   * 记录标记，供 onDrop 判断来源。
   */
  function onDragStart(_e: DragEvent): void {
    isInternalDrag = true
  }

  /** 拖拽操作结束（drop 后或取消后），清除内部拖拽标记。 */
  function onDragEnd(_e: DragEvent): void {
    isInternalDrag = false
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
    // 忽略窗口内部发起的拖拽（如选中文本后拖动），仅处理外部来源的拖入
    if (isInternalDrag) return
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
    window.addEventListener('dragstart', onDragStart)
    window.addEventListener('dragend', onDragEnd)
  })

  onUnmounted(() => {
    window.removeEventListener('paste', onPaste)
    window.removeEventListener('drop', onDrop)
    window.removeEventListener('dragover', onDragOver)
    window.removeEventListener('dragstart', onDragStart)
    window.removeEventListener('dragend', onDragEnd)
  })
}
