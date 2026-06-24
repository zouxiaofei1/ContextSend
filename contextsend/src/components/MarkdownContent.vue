<script setup lang="ts">
// 渲染一条消息的 content：纯文本走 Markdown；多模态数组中 text 块走 Markdown、
// image_url 块渲染 <img>。内容来自对端，HTML 经 DOMPurify 净化（见 useMarkdown）。
import { computed } from 'vue'
import { renderMarkdown } from '../composables/useMarkdown'

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

/** 归一化为内容块数组，便于模板统一遍历。 */
const parts = computed<ContentPart[]>(() => {
  const c = props.content
  if (typeof c === 'string') return [{ type: 'text', text: c }]
  if (Array.isArray(c)) {
    return c.filter(
      (p): p is ContentPart =>
        !!p && (p.type === 'text' || p.type === 'image_url'),
    )
  }
  return [{ type: 'text', text: '[不支持的内容]' }]
})

function html(text: string): string {
  return renderMarkdown(text)
}
</script>

<template>
  <div class="md-content">
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
.md-body :deep(pre.hljs) {
  background: var(--bg-tertiary, #1e1e1e);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 0.75rem 0.9rem;
  overflow-x: auto;
  margin: 0.5rem 0;
  font-size: 0.82rem;
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
</style>
