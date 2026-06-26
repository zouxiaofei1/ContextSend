<script setup lang="ts">
// 单段对话卡片：头部（标题/来源/计数）+ 展开后的消息体 + 操作区。
// 操作区为「编辑 / 复制 / 删除 / 更多」，更多下拉含设为推送源、导入到 Jan / ChatBox。
// - 复制：拼成「title:… / role:content …」的纯文本。
// - 编辑：弹出悬浮窗，Title 与每条消息各一个输入框，保存回写该段对话。
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useAppStore, type ChatMessage, type ConversationSegment } from '../stores/app'
import { useToastStore } from '../stores/toast'
import { ADAPTER_JAN, ADAPTER_CHATBOX } from '../constants'
import MarkdownContent from './MarkdownContent.vue'

const props = defineProps<{ segment: ConversationSegment; expanded: boolean; unread: boolean }>()
defineEmits<{ toggle: [] }>()

const app = useAppStore()
const toast = useToastStore()
const { t } = useI18n()

const showMenu = ref(false)
const editing = ref(false)
const editTitle = ref('')
const editContents = ref<string[]>([])
const menuWrap = ref<HTMLElement | null>(null)

const isPushSource = computed(() => app.selectedSegmentId === props.segment.id)

const segTitle = computed(
  () => props.segment.conversation.title || props.segment.conversation.model || t('receive.title'),
)

function fmtTime(ts: number): string {
  const d = new Date(ts)
  const pad = (n: number) => String(n).padStart(2, '0')
  return `${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`
}

/** 取消息可编辑文本：多模态时合并所有 text 块。 */
function textOf(content: unknown): string {
  if (typeof content === 'string') return content
  if (Array.isArray(content)) {
    return content
      .filter(
        (p): p is { type: 'text'; text: string } =>
          !!p && p.type === 'text' && typeof p.text === 'string',
      )
      .map((p) => p.text)
      .join('\n')
  }
  return ''
}

/** 用编辑文本重建 content：字符串原样替换；多模态保留图片块、替换文本块。 */
function rebuildContent(orig: unknown, text: string): unknown {
  if (Array.isArray(orig)) {
    const images = orig.filter((p) => !!p && (p as { type?: string }).type === 'image_url')
    const parts: unknown[] = []
    if (text) parts.push({ type: 'text', text })
    parts.push(...images)
    return parts.length ? parts : text
  }
  return text
}

/** 复制为纯文本：title 行 + 每条 role:content。 */
async function copyConversation(): Promise<void> {
  const conv = props.segment.conversation
  const lines: string[] = []
  if (conv.title) lines.push(`title:${conv.title}`)
  for (const m of conv.messages) lines.push(`${m.role}:${textOf(m.content)}`)
  try {
    await navigator.clipboard.writeText(lines.join('\n'))
    toast.success(t('receive.conversationCopied'))
  } catch (e) {
    toast.error(String(e))
  }
}

/** 打开编辑悬浮窗，预填 title 与每条消息文本。 */
function openEdit(): void {
  const conv = props.segment.conversation
  editTitle.value = conv.title ?? ''
  editContents.value = conv.messages.map((m) => textOf(m.content))
  editing.value = true
  showMenu.value = false
}

function saveEdit(): void {
  const conv = props.segment.conversation
  const messages: ChatMessage[] = conv.messages.map((m, i) => ({
    ...m,
    content: rebuildContent(m.content, editContents.value[i] ?? ''),
  }))
  app.updateSegmentConversation(props.segment.id, {
    ...conv,
    title: editTitle.value.trim() || undefined,
    messages,
  })
  editing.value = false
  toast.success(t('receive.editSaved'))
}

function cancelEdit(): void {
  editing.value = false
}

function remove(): void {
  app.removeSegment(props.segment.id)
}

function setPushSource(): void {
  app.selectSegment(props.segment.id)
  showMenu.value = false
}

function importJan(): void {
  app.importToApp(props.segment.conversation, ADAPTER_JAN)
  showMenu.value = false
}

function importChatBox(): void {
  app.importToApp(props.segment.conversation, ADAPTER_CHATBOX)
  showMenu.value = false
}

// 点击下拉菜单外部时关闭。
function onDocClick(e: MouseEvent): void {
  if (showMenu.value && menuWrap.value && !menuWrap.value.contains(e.target as Node)) {
    showMenu.value = false
  }
}
onMounted(() => document.addEventListener('click', onDocClick))
onBeforeUnmount(() => document.removeEventListener('click', onDocClick))
</script>

<template>
  <li class="seg-item" :class="{ 'seg-item--unread': unread }">
    <div class="seg-head" @click="$emit('toggle')">
      <span class="seg-toggle muted">{{ expanded ? '▲' : '▼' }}</span>
      <span class="seg-title">{{ segTitle }}</span>
      <span class="seg-meta muted">
        {{ segment.fromName }} · {{ fmtTime(segment.receivedAt) }} ·
        {{ t('receive.count', { count: segment.conversation.messages.length }) }}
      </span>
      <span v-if="unread" class="dot-new" />
      <span v-else-if="isPushSource" class="push-badge">{{ t('receive.pushSource') }}</span>
    </div>

    <div v-if="expanded" class="seg-body">
      <div v-for="(m, i) in segment.conversation.messages" :key="i" class="msg">
        <b class="msg-role">{{ m.role }}</b>
        <MarkdownContent :content="m.content" />
      </div>
      <div class="seg-actions">
        <button class="small ghost" @click="openEdit">{{ t('receive.edit') }}</button>
        <button class="small ghost" @click="copyConversation">{{ t('receive.copy') }}</button>
        <button class="small ghost danger" @click="remove">{{ t('receive.delete') }}</button>
        <div ref="menuWrap" class="more-wrap">
          <button class="small ghost" @click="showMenu = !showMenu">{{ t('receive.more') }} ▾</button>
          <div v-if="showMenu" class="more-menu">
            <button class="menu-item" @click="setPushSource">{{ t('receive.setPushSource') }}</button>
            <button class="menu-item" @click="importJan">{{ t('receive.importToJan') }}</button>
            <button class="menu-item" @click="importChatBox">
              {{ t('receive.importToChatBox') }}
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- 编辑悬浮窗 -->
    <Teleport to="body">
      <div v-if="editing" class="edit-overlay" @click.self="cancelEdit">
        <div class="edit-dialog">
          <h3 class="edit-dialog__title">{{ t('receive.editConversation') }}</h3>
          <div class="edit-scroll">
            <div class="edit-field">
              <span class="edit-label">{{ t('receive.titleField') }}</span>
              <input v-model="editTitle" class="edit-input" type="text" />
            </div>
            <div v-for="(m, i) in segment.conversation.messages" :key="i" class="edit-field">
              <span class="edit-label">{{ m.role }}</span>
              <textarea v-model="editContents[i]" class="edit-textarea" rows="4" spellcheck="false" />
            </div>
          </div>
          <div class="edit-actions">
            <button class="small ghost" @click="cancelEdit">{{ t('receive.cancel') }}</button>
            <button class="small" @click="saveEdit">{{ t('receive.save') }}</button>
          </div>
        </div>
      </div>
    </Teleport>
  </li>
</template>

<style scoped>
.seg-item {
  border-bottom: 1px solid var(--border);
}
.seg-item:last-child {
  border-bottom: none;
}

.seg-head {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.5rem 0;
  cursor: pointer;
}
.seg-head:hover {
  color: var(--accent);
}

.seg-toggle {
  font-size: 0.65rem;
  flex-shrink: 0;
}

.seg-title {
  font-weight: 600;
  font-size: 0.92rem;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.seg-meta {
  font-size: 0.75rem;
  margin-left: auto;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 40%;
  flex-shrink: 1;
}

.dot-new {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--accent);
  flex-shrink: 0;
}

.push-badge {
  font-size: 0.65rem;
  color: var(--accent);
  border: 1px solid var(--accent);
  border-radius: 4px;
  padding: 0.05rem 0.3rem;
  flex-shrink: 0;
}

.seg-body {
  padding: 0.25rem 0 0.75rem;
}

.msg {
  padding: 0.5rem 0;
  border-top: 1px dashed var(--border);
}

.msg-role {
  display: block;
  font-size: 0.78rem;
  color: var(--accent);
  text-transform: uppercase;
  margin-bottom: 0.25rem;
}

.seg-actions {
  display: flex;
  gap: 0.5rem;
  margin-top: 0.5rem;
  flex-wrap: wrap;
}

.danger:hover {
  color: #e5534b;
}

/* 更多下拉 */
.more-wrap {
  position: relative;
}

.more-menu {
  position: absolute;
  right: 0;
  bottom: calc(100% + 0.25rem);
  z-index: 20;
  display: flex;
  flex-direction: column;
  min-width: 9rem;
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 0.25rem;
  box-shadow: 0 6px 20px rgba(0, 0, 0, 0.25);
}

.menu-item {
  text-align: left;
  background: transparent;
  border: none;
  color: var(--text-primary);
  font-size: 0.82rem;
  padding: 0.4rem 0.55rem;
  border-radius: 6px;
  cursor: pointer;
  white-space: nowrap;
}
.menu-item:hover {
  background: rgba(127, 127, 127, 0.12);
  color: var(--accent);
}

/* 编辑悬浮窗 */
.edit-overlay {
  position: fixed;
  inset: 0;
  z-index: 1000;
  background: rgba(0, 0, 0, 0.45);
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 1.5rem;
}

.edit-dialog {
  width: min(680px, 100%);
  max-height: 85vh;
  display: flex;
  flex-direction: column;
  background: var(--bg-primary);
  border: 1px solid var(--border);
  border-radius: 12px;
  box-shadow: 0 16px 48px rgba(0, 0, 0, 0.4);
  overflow: hidden;
}

.edit-dialog__title {
  margin: 0;
  padding: 1rem 1.25rem;
  font-size: 1rem;
  border-bottom: 1px solid var(--border);
}

.edit-scroll {
  padding: 1rem 1.25rem;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 0.85rem;
}

.edit-field {
  display: flex;
  flex-direction: column;
  gap: 0.3rem;
}

.edit-label {
  font-size: 0.72rem;
  color: var(--accent);
  text-transform: uppercase;
  letter-spacing: 0.03em;
}

.edit-input,
.edit-textarea {
  width: 100%;
  font-size: 0.85rem;
}

.edit-textarea {
  resize: vertical;
  font-family: inherit;
  line-height: 1.5;
}

.edit-actions {
  display: flex;
  justify-content: flex-end;
  gap: 0.5rem;
  padding: 0.85rem 1.25rem;
  border-top: 1px solid var(--border);
}
</style>
