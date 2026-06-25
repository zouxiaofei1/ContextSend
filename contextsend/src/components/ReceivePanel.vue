<script setup lang="ts">
import { ref, computed } from 'vue'
import { useAppStore, type ConversationSegment } from '../stores/app'
import { useToastStore } from '../stores/toast'
import { useI18n } from 'vue-i18n'
import MarkdownContent from './MarkdownContent.vue'

const app = useAppStore()
const toast = useToastStore()
const { t } = useI18n()

const importText = ref('')
const exportText = ref('')

/** 已展开的段 id 集合。 */
const expandedSegments = ref<Set<string>>(new Set())
/** 分组折叠状态（默认未读展开、已读收起）。 */
const groupCollapsed = ref<{ unread: boolean; read: boolean }>({ unread: false, read: true })
/** 导入/导出区域是否展开 */
const showImportExport = ref(false)

const unreadSegments = computed(() => app.segments.filter((s) => !s.read))
const readSegments = computed(() => app.segments.filter((s) => s.read))

function segTitle(s: ConversationSegment): string {
  return s.conversation.title || s.conversation.model || t('receive.title')
}

function fmtTime(ts: number): string {
  const d = new Date(ts)
  const pad = (n: number) => String(n).padStart(2, '0')
  return `${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`
}

function toggleSegment(s: ConversationSegment): void {
  if (expandedSegments.value.has(s.id)) {
    expandedSegments.value.delete(s.id)
  } else {
    expandedSegments.value.add(s.id)
    // 展开未读段即标记为已读（自动归入已读组）
    if (!s.read) app.markRead(s.id)
  }
}

function isExpanded(id: string): boolean {
  return expandedSegments.value.has(id)
}

function onClear(): void {
  if (confirm(t('receive.confirmClear'))) {
    app.clearSegments()
    expandedSegments.value.clear()
  }
}

async function onImport(): Promise<void> {
  try {
    const conv = await app.importOpenai(importText.value)
    app.addSegment(t('receive.localImport'), conv, true)
    importText.value = ''
    toast.success(t('receive.importSuccess', { count: conv.messages.length }))
  } catch (e) {
    toast.error(String(e))
  }
}

async function onExport(): Promise<void> {
  const seg = app.segments.find((s) => s.id === app.selectedSegmentId) ?? app.segments[0]
  if (!seg) return
  try {
    exportText.value = await app.exportOpenai(seg.conversation)
    toast.success(t('receive.exportReady'))
  } catch (e) {
    toast.error(String(e))
  }
}
</script>

<template>
  <div class="panel">
    <!-- 顶部操作条 -->
    <div class="toolbar">
      <h2>{{ t('receive.title') }}</h2>
      <div class="toolbar__actions" v-if="app.segments.length">
        <button class="small ghost" @click="app.markAllRead()">
          {{ t('receive.markAllRead') }}
        </button>
        <button class="small ghost" @click="onClear()">{{ t('receive.clear') }}</button>
      </div>
    </div>

    <p v-if="app.segments.length === 0" class="muted empty">{{ t('receive.emptySegments') }}</p>

    <p class="muted capture-hint">{{ t('receive.captureHint') }}</p>

    <!-- 未读分组 -->
    <section v-if="unreadSegments.length" class="card">
      <button class="ghost section-toggle" @click="groupCollapsed.unread = !groupCollapsed.unread">
        {{ groupCollapsed.unread ? '▶' : '▼' }} {{ t('receive.unread') }} ({{
          unreadSegments.length
        }})
      </button>
      <ul v-show="!groupCollapsed.unread" class="seg-list">
        <li v-for="s in unreadSegments" :key="s.id" class="seg-item seg-item--unread">
          <div class="seg-head" @click="toggleSegment(s)">
            <span class="seg-toggle muted">{{ isExpanded(s.id) ? '▲' : '▼' }}</span>
            <span class="seg-title">{{ segTitle(s) }}</span>
            <span class="seg-meta muted">
              {{ s.fromName }} · {{ fmtTime(s.receivedAt) }} ·
              {{ t('receive.count', { count: s.conversation.messages.length }) }}
            </span>
            <span class="dot-new" />
          </div>
          <div v-if="isExpanded(s.id)" class="seg-body">
            <div v-for="(m, i) in s.conversation.messages" :key="i" class="msg">
              <b class="msg-role">{{ m.role }}</b>
              <MarkdownContent :content="m.content" />
            </div>
            <div class="seg-actions">
              <button class="small ghost" @click="app.selectSegment(s.id)">
                {{ t('receive.setPushSource') }}
              </button>
              <button class="small ghost" @click="app.importToApp(s.conversation, 'Jan')">
                {{ t('receive.importToJan') }}
              </button>
              <button class="small ghost" @click="app.importToApp(s.conversation, 'ChatBox')">
                {{ t('receive.importToChatBox') }}
              </button>
              <button class="small ghost danger" @click="app.removeSegment(s.id)">
                {{ t('receive.delete') }}
              </button>
            </div>
          </div>
        </li>
      </ul>
    </section>

    <!-- 已读分组 -->
    <section v-if="readSegments.length" class="card">
      <button class="ghost section-toggle" @click="groupCollapsed.read = !groupCollapsed.read">
        {{ groupCollapsed.read ? '▶' : '▼' }} {{ t('receive.read') }} ({{ readSegments.length }})
      </button>
      <ul v-show="!groupCollapsed.read" class="seg-list">
        <li v-for="s in readSegments" :key="s.id" class="seg-item">
          <div class="seg-head" @click="toggleSegment(s)">
            <span class="seg-toggle muted">{{ isExpanded(s.id) ? '▲' : '▼' }}</span>
            <span class="seg-title">{{ segTitle(s) }}</span>
            <span class="seg-meta muted">
              {{ s.fromName }} · {{ fmtTime(s.receivedAt) }} ·
              {{ t('receive.count', { count: s.conversation.messages.length }) }}
            </span>
            <span v-if="s.id === app.selectedSegmentId" class="push-badge">{{
              t('receive.pushSource')
            }}</span>
          </div>
          <div v-if="isExpanded(s.id)" class="seg-body">
            <div v-for="(m, i) in s.conversation.messages" :key="i" class="msg">
              <b class="msg-role">{{ m.role }}</b>
              <MarkdownContent :content="m.content" />
            </div>
            <div class="seg-actions">
              <button class="small ghost" @click="app.selectSegment(s.id)">
                {{ t('receive.setPushSource') }}
              </button>
              <button class="small ghost" @click="app.importToApp(s.conversation, 'Jan')">
                {{ t('receive.importToJan') }}
              </button>
              <button class="small ghost" @click="app.importToApp(s.conversation, 'ChatBox')">
                {{ t('receive.importToChatBox') }}
              </button>
              <button class="small ghost danger" @click="app.removeSegment(s.id)">
                {{ t('receive.delete') }}
              </button>
            </div>
          </div>
        </li>
      </ul>
    </section>

    <!-- 导入导出的折叠区 -->
    <section class="card">
      <button class="ghost section-toggle" @click="showImportExport = !showImportExport">
        {{ showImportExport ? '▼' : '▶' }} {{ t('receive.sectionImportExport') }}
      </button>

      <template v-if="showImportExport">
        <textarea
          v-model="importText"
          rows="4"
          :placeholder="t('receive.importPlaceholder')"
          style="margin-top: 0.5rem"
        />
        <div class="row">
          <button @click="onImport">{{ t('receive.import') }}</button>
          <button @click="onExport">{{ t('receive.export') }}</button>
        </div>
        <textarea
          v-if="exportText"
          :value="exportText"
          rows="6"
          readonly
          style="margin-top: 0.5rem"
        />
      </template>
    </section>
  </div>
</template>

<style scoped>
.panel {
  flex: 1;
  padding: 1.5rem;
  overflow-y: auto;
}

.toolbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 0.75rem;
}

.toolbar h2 {
  margin: 0;
  font-size: 1.05rem;
}

.toolbar__actions {
  display: flex;
  gap: 0.5rem;
}

.empty {
  padding: 1.5rem 0;
  text-align: center;
}

.capture-hint {
  font-size: 0.78rem;
  margin: 0 0 0.75rem;
}

.card {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 1rem 1.25rem;
  margin-bottom: 1rem;
}

.row {
  display: flex;
  gap: 0.5rem;
  margin-top: 0.5rem;
  flex-wrap: wrap;
}

.section-toggle {
  width: 100%;
  text-align: left;
  background: transparent;
  color: var(--text-primary);
  font-size: 0.95rem;
  padding: 0;
  border: none;
  cursor: pointer;
}

.section-toggle:hover {
  color: var(--accent);
  background: transparent;
}

/* 段列表 */
.seg-list {
  margin: 0.5rem 0 0;
  padding-left: 0;
  list-style: none;
}

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
  flex-shrink: 0;
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

/* 段展开后的消息体 */
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
}

.danger:hover {
  color: #e5534b;
}

textarea {
  resize: vertical;
}
</style>
