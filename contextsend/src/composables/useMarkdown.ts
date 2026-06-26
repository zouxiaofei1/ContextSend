// Markdown 渲染：markdown-it 解析 + highlight.js 代码高亮 + DOMPurify 净化。
//
// 内容来自局域网对端，**必须净化**后才能 v-html 输出，防止 XSS。
// markdown-it / highlight.js 主题在模块级单例创建，避免每条消息重复构造。

import MarkdownIt from 'markdown-it'
import hljs from 'highlight.js'
import DOMPurify from 'dompurify'
import markdownItKatex from '@vscode/markdown-it-katex'
import 'highlight.js/styles/github-dark.css'
import 'katex/dist/katex.min.css'

// 该插件为 CJS：不同打包器的 ESM 互操作下，默认导出可能多包若干层 default。
// 循环解包到真正的函数，杜绝 node/esbuild/vite 行为差异导致的 md.use 运行时报错。
function unwrapDefault(m: unknown): unknown {
  let cur = m
  while (cur && typeof cur === 'object' && 'default' in cur) {
    cur = (cur as { default: unknown }).default
  }
  return cur
}
const katexPlugin = unwrapDefault(markdownItKatex) as Parameters<MarkdownIt['use']>[0]

const md = new MarkdownIt({
  linkify: true,
  breaks: true,
  highlight(code: string, lang: string): string {
    // 仅输出高亮后的静态 HTML 并带上语言标签；行号、复制按钮等交互
    // 由 MarkdownContent 在挂载后基于 data-lang 增强，避免 v-html 内绑事件。
    const language = lang && hljs.getLanguage(lang) ? lang : ''
    let html: string
    if (language) {
      try {
        html = hljs.highlight(code, { language, ignoreIllegals: true }).value
      } catch {
        html = md.utils.escapeHtml(code)
      }
    } else {
      html = md.utils.escapeHtml(code)
    }
    // 用原始 lang 作标签：即使 hljs 未高亮（mermaid / dockerfile 等），也保留语言
    // 标识，供前端显示图标名、以及识别 mermaid 做图形渲染。转义防属性注入。
    const label = lang || 'text'
    return `<pre class="code-block hljs" data-lang="${md.utils.escapeHtml(label)}"><code>${html}</code></pre>`
  },
})

// 数学公式：$...$ 行内、$$...$$ 块级，KaTeX 渲染。throwOnError 关闭以便
// 非法公式回退为红色源码而非抛异常打断整段渲染。
md.use(katexPlugin, { throwOnError: false, errorColor: '#e5534b' })

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
  // ADD_ATTR.target：保留我们为链接添加的 target；
  // semantics/annotation/encoding：保留 KaTeX 的 MathML 语义层（屏幕阅读器用，
  // 视觉渲染靠 HTML span，二者默认即放行 style/class）。
  return DOMPurify.sanitize(md.render(text), {
    ADD_ATTR: ['target', 'encoding'],
    ADD_TAGS: ['semantics', 'annotation'],
  })
}

/**
 * 净化一段 SVG 源码用于安全内联渲染（代码块预览）。
 *
 * 内容来自对端，SVG 可携带 `<script>` / `on*` 事件 / `<foreignObject>` 等攻击面。
 * 用 DOMPurify 的 SVG 配置只保留绘图相关标签与属性，移除脚本与事件处理器。
 * 返回空串表示无有效 SVG（调用方应回退为源码展示）。
 */
export function sanitizeSvg(src: string): string {
  return DOMPurify.sanitize(src, { USE_PROFILES: { svg: true, svgFilters: true } })
}
