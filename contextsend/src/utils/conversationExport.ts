// 对话导出：把一段 Conversation 转成 OpenAI JSON / Markdown / HTML 文本，
// 经 plugin-dialog「另存为」选路径后由后端 `save_export` 写盘；PDF 走浏览器
// 打印另存（零依赖、中文完美），由 printConversation 弹出系统打印框。
import { invoke } from '@tauri-apps/api/core'
import { save } from '@tauri-apps/plugin-dialog'
import { IPC } from '../constants'
import { renderMarkdown } from '../composables/useMarkdown'
import type { Conversation } from '../stores/types'

/** 导出格式标识。 */
export type ExportFormat = 'json' | 'markdown' | 'html' | 'pdf'

/** 取消息纯文本：多模态时合并所有 text 块，图片块以占位符表示。 */
function textOf(content: unknown): string {
  if (typeof content === 'string') return content
  if (Array.isArray(content)) {
    return content
      .map((p) => {
        if (!p || typeof p !== 'object') return ''
        const part = p as { type?: string; text?: string }
        if (part.type === 'text' && typeof part.text === 'string') return part.text
        if (part.type === 'image_url') return '[图片]'
        return ''
      })
      .filter(Boolean)
      .join('\n')
  }
  return ''
}

/** 导出标题（缺省回退到 model / 通用名）。 */
function titleOf(conv: Conversation): string {
  return conv.title || conv.model || '对话'
}

/** OpenAI Chat Completion 风格 JSON：{ model?, messages:[{role,content}] }。 */
function toOpenAiJson(conv: Conversation): string {
  const payload: { model?: string; messages: { role: string; content: string }[] } = {
    messages: conv.messages.map((m) => ({ role: m.role, content: textOf(m.content) })),
  }
  if (conv.model) payload.model = conv.model
  return JSON.stringify(payload, null, 2)
}

/** Markdown：# 标题 + 每条 **role** 段落。 */
function toMarkdown(conv: Conversation): string {
  const lines: string[] = [`# ${titleOf(conv)}`, '']
  for (const m of conv.messages) {
    lines.push(`## ${m.role}`, '', textOf(m.content), '')
  }
  return lines.join('\n')
}

/** HTML 转义（标题等纯文本节点用）。 */
function escapeHtml(s: string): string {
  return s
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
}

/** 自包含 HTML 文档：复用 markdown 渲染（含代码高亮 / LaTeX），离线可看。 */
function toHtml(conv: Conversation): string {
  const title = escapeHtml(titleOf(conv))
  const body = conv.messages
    .map(
      (m) =>
        `<section class="msg"><div class="role">${escapeHtml(m.role)}</div>` +
        `<div class="content">${renderMarkdown(textOf(m.content))}</div></section>`,
    )
    .join('\n')
  return `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8" />
<meta name="viewport" content="width=device-width, initial-scale=1" />
<title>${title}</title>
<style>
  body { font-family: -apple-system, "Segoe UI", "Microsoft YaHei", sans-serif;
    max-width: 820px; margin: 2rem auto; padding: 0 1rem; line-height: 1.6; color: #1a1a1a; }
  h1 { font-size: 1.5rem; border-bottom: 1px solid #ddd; padding-bottom: .5rem; }
  .msg { margin: 1.25rem 0; }
  .role { font-size: .8rem; text-transform: uppercase; letter-spacing: .03em;
    color: #2563eb; font-weight: 600; margin-bottom: .35rem; }
  .content { white-space: normal; }
  pre { background: #0d1117; color: #e6edf3; padding: .85rem 1rem; border-radius: 8px;
    overflow-x: auto; }
  code { font-family: "SFMono-Regular", Consolas, monospace; font-size: .88em; }
  :not(pre) > code { background: rgba(127,127,127,.15); padding: .1em .35em; border-radius: 4px; }
  blockquote { border-left: 3px solid #ddd; margin: 0; padding-left: 1rem; color: #555; }
  table { border-collapse: collapse; } td, th { border: 1px solid #ddd; padding: .35rem .6rem; }
  @media print { body { margin: 0; max-width: none; } }
</style>
</head>
<body>
<h1>${title}</h1>
${body}
</body>
</html>`
}

/** 文件名安全化：去掉非法字符，限长。 */
function safeFileName(name: string): string {
  return (
    name
      .replace(/[\\/:*?"<>|\n\r\t]/g, '_')
      .replace(/\s+/g, ' ')
      .trim()
      .slice(0, 80) || '对话'
  )
}

const EXT: Record<Exclude<ExportFormat, 'pdf'>, { ext: string; name: string }> = {
  json: { ext: 'json', name: 'OpenAI JSON' },
  markdown: { ext: 'md', name: 'Markdown' },
  html: { ext: 'html', name: 'HTML' },
}

/** PDF：把 HTML 写进隐藏 iframe，调用其 print() 弹出系统打印框（选「另存为 PDF」）。 */
function printConversation(conv: Conversation): void {
  const iframe = document.createElement('iframe')
  iframe.style.position = 'fixed'
  iframe.style.right = '0'
  iframe.style.bottom = '0'
  iframe.style.width = '0'
  iframe.style.height = '0'
  iframe.style.border = '0'
  document.body.appendChild(iframe)
  const doc = iframe.contentDocument
  if (!doc) {
    document.body.removeChild(iframe)
    return
  }
  doc.open()
  doc.write(toHtml(conv))
  doc.close()
  const win = iframe.contentWindow
  if (!win) {
    document.body.removeChild(iframe)
    return
  }
  // 等样式 / 字体就绪再打印，打印结束后移除 iframe。
  const trigger = () => {
    win.focus()
    win.print()
    setTimeout(() => document.body.removeChild(iframe), 1000)
  }
  if (doc.readyState === 'complete') setTimeout(trigger, 100)
  else win.addEventListener('load', () => setTimeout(trigger, 100))
}

/**
 * 导出一段对话。json/markdown/html 弹「另存为」选路径并写盘，返回是否实际保存
 * （用户取消返回 false）；pdf 弹系统打印框由用户另存，立即返回 true。
 */
export async function exportConversation(
  conv: Conversation,
  format: ExportFormat,
): Promise<boolean> {
  if (format === 'pdf') {
    printConversation(conv)
    return true
  }

  const meta = EXT[format]
  const defaultPath = `${safeFileName(titleOf(conv))}.${meta.ext}`
  const path = await save({
    defaultPath,
    filters: [{ name: meta.name, extensions: [meta.ext] }],
  })
  if (!path) return false

  const contents =
    format === 'json' ? toOpenAiJson(conv) : format === 'markdown' ? toMarkdown(conv) : toHtml(conv)
  await invoke(IPC.SAVE_EXPORT, { path, contents })
  return true
}
