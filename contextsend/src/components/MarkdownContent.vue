<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { renderMarkdown, sanitizeSvg } from '../composables/useMarkdown'
import { resolveLang, fallbackIconSvg } from '../composables/useLangIcon'

const { t, locale } = useI18n()

/** 超过此行数的代码块默认折叠。 */
const FOLD_LINES = 24
/** 折叠后仍显示前 N 行作为预览。 */
const PREVIEW_LINES = 10

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
  code.innerHTML = code.innerHTML.replace(/\n+$/, '')

  const raw = code.textContent ?? ''
  const lang = pre.dataset.lang || 'text'

  // 头部：语言图标 + 名 + 复制按钮
  const header = document.createElement('div')
  header.className = 'code-header'
  const langEl = document.createElement('span')
  langEl.className = 'code-lang'
  const { name, iconUrl, abbr, hue } = resolveLang(lang)
  const iconBox = document.createElement('span')
  iconBox.className = 'code-lang-icon'
  if (iconUrl) {
    const img = document.createElement('img')
    img.src = iconUrl
    img.alt = ''
    iconBox.append(img)
  } else {
    // 未收录图标：圆角矩形 + 右下角缩写占位
    iconBox.innerHTML = fallbackIconSvg(abbr, hue)
  }
  langEl.append(iconBox, document.createTextNode(name))
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
  // 折叠按钮 + 复制按钮归入右侧操作区
  const toggle = document.createElement('button')
  toggle.className = 'code-fold'
  toggle.type = 'button'
  toggle.title = t('receive.toggleFold')
  toggle.setAttribute('aria-label', t('receive.toggleFold'))
  const actions = document.createElement('div')
  actions.className = 'code-actions'
  actions.append(toggle, copyBtn)
  header.append(langEl, actions)

  // 行号 gutter（不参与复制）+ 可横向滚动的代码区
  const lineCount = raw.split('\n').length
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

  // 代码折叠：超过阈值的长代码默认折叠，显示前 PREVIEW_LINES 行预览；
  // 点击头部箭头切换展开/折叠。
  let collapsed = lineCount > FOLD_LINES
  const applyFold = (): void => {
    if (collapsed) {
      area.style.maxHeight = `calc(${PREVIEW_LINES} * 1.5em + 1.5rem)`
      area.style.overflow = 'hidden'
    } else {
      area.style.maxHeight = ''
      area.style.overflow = ''
    }
    toggle.textContent = collapsed ? '▸' : '▾'
    toggle.setAttribute('aria-expanded', String(!collapsed))
  }
  toggle.addEventListener('click', () => {
    collapsed = !collapsed
    applyFold()
  })
  applyFold()

  pre.append(header, area)
}

/**
 * 渲染 mermaid 图：懒加载 mermaid（体积大），strict 模式（内容来自对端，禁交互并
 * 净化）。渲染失败则还原为普通代码块展示源码。
 */
async function renderMermaid(pre: HTMLElement): Promise<void> {
  const src = (pre.querySelector('code')?.textContent ?? '').trim()
  if (!src) {
    enhanceBlock(pre)
    return
  }
  const holder = document.createElement('div')
  holder.className = 'mermaid-rendered'
  pre.replaceWith(holder)
  try {
    const mermaid = (await import('mermaid')).default
    const dark = document.documentElement.getAttribute('data-theme') === 'dark'
    mermaid.initialize({
      startOnLoad: false,
      securityLevel: 'strict',
      theme: dark ? 'dark' : 'default',
    })
    const { svg } = await mermaid.render(`csmmd-${crypto.randomUUID()}`, src)
    holder.innerHTML = svg
  } catch {
    holder.replaceWith(pre)
    enhanceBlock(pre)
  }
}

/** 判断代码块是否为可渲染的 SVG（明确 svg，或 xml/html/无标注但内容含完整 svg）。 */
function isSvgBlock(pre: HTMLElement): boolean {
  const lang = pre.dataset.lang ?? ''
  const src = pre.querySelector('code')?.textContent ?? ''
  if (lang === 'svg') return /<svg[\s>]/i.test(src)
  if (lang === 'xml' || lang === 'html' || lang === '' || lang === 'text') {
    return /<svg[\s\S]*<\/svg>/i.test(src)
  }
  return false
}

/**
 * SVG 代码块：在普通代码块基础上加「预览 / 源码」切换，默认显示净化后的渲染图。
 * 净化失败或无有效内容时保持源码视图。
 */
function renderSvgPreview(pre: HTMLElement): void {
  const rawSrc = pre.querySelector('code')?.textContent ?? ''
  enhanceBlock(pre) // 先生成源码视图（header / 行号 / 复制）
  const clean = sanitizeSvg(rawSrc)
  const header = pre.querySelector<HTMLElement>('.code-header')
  const area = pre.querySelector<HTMLElement>('.code-area')
  const actions = header?.querySelector<HTMLElement>('.code-actions')
  if (!clean.trim() || !header || !area || !actions) return

  const preview = document.createElement('div')
  preview.className = 'svg-preview'
  preview.innerHTML = clean

  const toggle = document.createElement('button')
  toggle.className = 'code-fold code-preview-toggle'
  toggle.type = 'button'
  let showSource = false
  const apply = (): void => {
    area.style.display = showSource ? '' : 'none'
    preview.style.display = showSource ? 'none' : ''
    toggle.textContent = showSource ? t('receive.preview') : t('receive.viewSource')
  }
  toggle.addEventListener('click', () => {
    showSource = !showSource
    apply()
  })
  actions.prepend(toggle)
  area.before(preview)
  apply() // 默认预览
}

/** 遍历容器下所有尚未处理的代码块：mermaid / SVG 走图形渲染，其余走代码增强。 */
function enhance(): void {
  const el = root.value
  if (!el) return
  el.querySelectorAll<HTMLElement>('pre.code-block:not([data-enhanced])').forEach((pre) => {
    pre.setAttribute('data-enhanced', '')
    if (pre.dataset.lang === 'mermaid') {
      void renderMermaid(pre)
    } else if (isSvgBlock(pre)) {
      renderSvgPreview(pre)
    } else {
      enhanceBlock(pre)
    }
  })
}
function refreshI18n(): void {
  const el = root.value
  if (!el) return
  el.querySelectorAll<HTMLElement>('pre.code-block[data-enhanced]').forEach((pre) => {
    const copyBtn = pre.querySelector<HTMLElement>('.code-copy')
    if (copyBtn) copyBtn.textContent = t('receive.copyCode')

    // Fold/collapse button: update title and aria-label (chevron is locale-independent)
    const foldBtn = pre.querySelector<HTMLElement>('.code-fold:not(.code-preview-toggle)')
    if (foldBtn) {
      foldBtn.title = t('receive.toggleFold')
      foldBtn.setAttribute('aria-label', t('receive.toggleFold'))
    }

    // SVG preview/source toggle: read current view state from DOM
    const toggleBtn = pre.querySelector<HTMLElement>('.code-preview-toggle')
    if (toggleBtn) {
      const area = pre.querySelector<HTMLElement>('.code-area')
      const showSource = area && area.style.display !== 'none'
      toggleBtn.textContent = showSource
        ? t('receive.preview')
        : t('receive.viewSource')
    }
  })
}

watch(
  parts,
  () => {
    nextTick(enhance)
  },
  { immediate: true },
)

// Re-apply i18n text on already-enhanced DOM nodes when locale switches
watch(locale, () => {
  refreshI18n()
})

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
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
  font-size: 0.72rem;
  color: var(--text-secondary);
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
}
.md-body :deep(.code-lang-icon) {
  width: 15px;
  height: 15px;
  flex-shrink: 0;
  display: inline-flex;
}
.md-body :deep(.code-lang-icon img),
.md-body :deep(.code-lang-icon svg) {
  width: 100%;
  height: 100%;
  display: block;
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
.md-body :deep(.code-actions) {
  display: inline-flex;
  align-items: center;
  gap: 0.2rem;
}
.md-body :deep(.code-fold) {
  font-size: 0.7rem;
  line-height: 1;
  color: var(--text-secondary);
  background: transparent;
  border: none;
  cursor: pointer;
  padding: 0.1rem 0.35rem;
  border-radius: 4px;
}
.md-body :deep(.code-fold:hover) {
  color: var(--accent);
  background: rgba(127, 127, 127, 0.12);
}
/* mermaid 渲染结果 */
.md-body :deep(.mermaid-rendered) {
  margin: 0.6rem 0;
  display: flex;
  justify-content: center;
  overflow-x: auto;
}
.md-body :deep(.mermaid-rendered svg) {
  max-width: 100%;
  height: auto;
}
/* SVG 代码块预览 */
.md-body :deep(.code-preview-toggle) {
  font-size: 0.7rem;
}
.md-body :deep(.svg-preview) {
  margin: 0.5rem 0;
  padding: 0.75rem;
  background: #fff;
  border: 1px solid var(--border);
  border-radius: 8px;
  display: flex;
  justify-content: center;
  overflow: auto;
}
.md-body :deep(.svg-preview svg) {
  max-width: 100%;
  height: auto;
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
  font-size: inherit;
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
  font-size: inherit;
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
