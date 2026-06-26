<script setup lang="ts">
// 渲染一条消息的 content：纯文本走 Markdown；多模态数组中 text 块走 Markdown、
// image_url 块渲染 <img>。内容来自对端，HTML 经 DOMPurify 净化（见 useMarkdown）。
//
// 代码块的交互（语言标签 / 复制 / 行号）在挂载后基于 useMarkdown 输出的
// data-lang 占位结构做 DOM 增强，避免在 v-html 字符串里绑定事件。
import { computed, nextTick, onBeforeUnmount, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { renderMarkdown } from '../composables/useMarkdown'

const { t } = useI18n()

/** 多模态内容块，对齐 cs-core 的 ContentPart。 */
interface TextPart {
  type: 'text'
  text: string
}
interface ImagePart {
  type: 'image_url'
  image_url: { url: string; detail?: string }
}
type ContentPart = TextPart | ImagePart

const props = defineProps<{ content: unknown }>()

const root = ref<HTMLElement | null>(null)
/** 复制成功后用于恢复按钮文案的定时器集合，卸载时清理。 */
const timers = new Set<ReturnType<typeof setTimeout>>()

/** 归一化为内容块数组，便于模板统一遍历。 */
const parts = computed<ContentPart[]>(() => {
  const c = props.content
  if (typeof c === 'string') return [{ type: 'text', text: c }]
  if (Array.isArray(c)) {
    return c.filter((p): p is ContentPart => !!p && (p.type === 'text' || p.type === 'image_url'))
  }
  return [{ type: 'text', text: '[不支持的内容]' }]
})

function html(text: string): string {
  return renderMarkdown(text)
}

/** 为单个代码块注入头部（语言 + 复制）和行号 gutter。 */
function enhanceBlock(pre: HTMLElement): void {
  const code = pre.querySelector('code')
  if (!code) return
  pre.setAttribute('data-enhanced', '')

  const raw = code.textContent ?? ''
  const lang = pre.dataset.lang || 'text'

  // 头部：语言标签 + 复制按钮
  const header = document.createElement('div')
  header.className = 'code-header'
  const langEl = document.createElement('span')
  langEl.className = 'code-lang'
  langEl.textContent = lang
  const copyBtn = document.createElement('button')
  copyBtn.className = 'code-copy'
  copyBtn.type = 'button'
  copyBtn.textContent = t('receive.copyCode')
  copyBtn.addEventListener('click', () => {
    navigator.clipboard.writeText(raw).then(() => {
      copyBtn.textContent = t('receive.copied')
      const id = setTimeout(() => {
        copyBtn.textContent = t('receive.copyCode')
        timers.delete(id)
      }, 1500)
      timers.add(id)
    })
  })
  header.append(langEl, copyBtn)

  // 行号 gutter（不参与复制）+ 可横向滚动的代码区
  const lineCount = (raw.replace(/\n+$/, '') || '').split('\n').length
  const gutter = document.createElement('div')
  gutter.className = 'code-gutter'
  gutter.setAttribute('aria-hidden', 'true')
  gutter.textContent = Array.from({ length: lineCount }, (_, i) => i + 1).join('\n')

  const scroll = document.createElement('div')
  scroll.className = 'code-scroll'
  scroll.append(code) // 把 code 从 pre 移入滚动容器

  const area = document.createElement('div')
  area.className = 'code-area'
  area.append(gutter, scroll)

  pre.append(header, area)
}

/** 遍历容器下所有尚未处理的代码块。 */
function enhance(): void {
  const el = root.value
  if (!el) return
  el.querySelectorAll<HTMLElement>('pre.code-block:not([data-enhanced])').forEach(enhanceBlock)
}

watch(
  parts,
  () => {
    nextTick(enhance)
  },
  { immediate: true },
)

onBeforeUnmount(() => {
  timers.forEach(clearTimeout)
  timers.clear()
})
</script>

<template>
  <div ref="root" class="md-content">
    <template v-for="(p, i) in parts" :key="i">
      <!-- 文本块：净化后的 Markdown HTML -->
      <div v-if="p.type === 'text'" class="md-body" v-html="html(p.text)" />
      <!-- 图像块 -->
      <img
        v-else-if="p.type === 'image_url'"
        class="md-image"
        :src="p.image_url.url"
        alt="图片"
        loading="lazy"
      />
    </template>
  </div>
</template>

<style scoped>
.md-content {
  font-size: 0.9rem;
  line-height: 1.55;
  word-break: break-word;
}

.md-image {
  max-width: 100%;
  border-radius: 6px;
  margin: 0.35rem 0;
}

/* Markdown 正文排版（深穿透到 v-html 内容） */
.md-body :deep(p) {
  margin: 0.4rem 0;
}
.md-body :deep(p:first-child) {
  margin-top: 0;
}
.md-body :deep(p:last-child) {
  margin-bottom: 0;
}
.md-body :deep(pre.code-block) {
  background: var(--bg-tertiary, #1e1e1e);
  border: 1px solid var(--border);
  border-radius: 8px;
  overflow: hidden;
  margin: 0.5rem 0;
  font-size: 0.82rem;
  padding: 0;
}

/* 代码块头部：语言标签 + 复制按钮 */
.md-body :deep(.code-header) {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.25rem 0.6rem;
  border-bottom: 1px solid var(--border);
  background: rgba(127, 127, 127, 0.08);
}
.md-body :deep(.code-lang) {
  font-size: 0.7rem;
  text-transform: uppercase;
  letter-spacing: 0.03em;
  color: var(--text-secondary);
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
}
.md-body :deep(.code-copy) {
  font-size: 0.7rem;
  color: var(--text-secondary);
  background: transparent;
  border: none;
  cursor: pointer;
  padding: 0.1rem 0.4rem;
  border-radius: 4px;
}
.md-body :deep(.code-copy:hover) {
  color: var(--accent);
  background: rgba(127, 127, 127, 0.12);
}

/* 行号 gutter + 横向滚动代码区 */
.md-body :deep(.code-area) {
  display: flex;
  align-items: stretch;
}
.md-body :deep(.code-gutter) {
  flex-shrink: 0;
  white-space: pre;
  text-align: right;
  user-select: none;
  color: var(--text-secondary);
  opacity: 0.5;
  padding: 0.75rem 0.5rem 0.75rem 0.9rem;
  border-right: 1px solid var(--border);
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  line-height: 1.5;
}
.md-body :deep(.code-scroll) {
  flex: 1;
  overflow-x: auto;
}
.md-body :deep(pre.code-block code) {
  display: block;
  white-space: pre;
  padding: 0.75rem 0.9rem;
  line-height: 1.5;
}
.md-body :deep(code) {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
}
/* 行内 code（非代码块内） */
.md-body :deep(:not(pre) > code) {
  background: var(--bg-tertiary, rgba(127, 127, 127, 0.15));
  padding: 0.1rem 0.35rem;
  border-radius: 4px;
  font-size: 0.85em;
}
.md-body :deep(a) {
  color: var(--accent);
}
.md-body :deep(blockquote) {
  border-left: 3px solid var(--border);
  margin: 0.5rem 0;
  padding-left: 0.8rem;
  color: var(--text-secondary);
}
.md-body :deep(table) {
  border-collapse: collapse;
  margin: 0.5rem 0;
}
.md-body :deep(th),
.md-body :deep(td) {
  border: 1px solid var(--border);
  padding: 0.3rem 0.6rem;
}
.md-body :deep(ul),
.md-body :deep(ol) {
  padding-left: 1.4rem;
  margin: 0.4rem 0;
}
.md-body :deep(img) {
  max-width: 100%;
}
/* 块级数学公式：长公式可横向滚动，避免撑破卡片 */
.md-body :deep(.katex-display) {
  overflow-x: auto;
  overflow-y: hidden;
  padding: 0.2rem 0;
}
</style>
