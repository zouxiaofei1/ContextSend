// Markdown 渲染：markdown-it 解析 + highlight.js 代码高亮 + DOMPurify 净化。
//
// 内容来自局域网对端，**必须净化**后才能 v-html 输出，防止 XSS。
// markdown-it / highlight.js 主题在模块级单例创建，避免每条消息重复构造。

import MarkdownIt from 'markdown-it'
import hljs from 'highlight.js'
import DOMPurify from 'dompurify'
import 'highlight.js/styles/github-dark.css'

const md = new MarkdownIt({
  linkify: true,
  breaks: true,
  highlight(code: string, lang: string): string {
    if (lang && hljs.getLanguage(lang)) {
      try {
        const html = hljs.highlight(code, { language: lang, ignoreIllegals: true }).value
        return `<pre class="hljs"><code data-lang="${lang}">${html}</code></pre>`
      } catch {
        /* 回退到转义纯文本 */
      }
    }
    const escaped = md.utils.escapeHtml(code)
    return `<pre class="hljs"><code>${escaped}</code></pre>`
  },
})

// 链接统一在新窗口打开并加 rel，避免 target=_blank 的安全问题。
const defaultLinkOpen =
  md.renderer.rules.link_open ??
  ((tokens, idx, options, _env, self) => self.renderToken(tokens, idx, options))
md.renderer.rules.link_open = (tokens, idx, options, env, self) => {
  tokens[idx].attrSet('target', '_blank')
  tokens[idx].attrSet('rel', 'noopener noreferrer')
  return defaultLinkOpen(tokens, idx, options, env, self)
}

// DOMPurify 钩子：保留我们添加的 target/rel。
DOMPurify.addHook('afterSanitizeAttributes', (node) => {
  if (node.tagName === 'A' && node.getAttribute('target') === '_blank') {
    node.setAttribute('rel', 'noopener noreferrer')
  }
})

/** 把 Markdown 文本渲染为净化后的安全 HTML。 */
export function renderMarkdown(text: string): string {
  return DOMPurify.sanitize(md.render(text), { ADD_ATTR: ['target'] })
}
