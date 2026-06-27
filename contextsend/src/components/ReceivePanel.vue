<script setup lang="ts">
import { ref, computed } from 'vue'
import { useAppStore, type ConversationSegment } from '../stores/app'
import { useToastStore } from '../stores/toast'
import { useI18n } from 'vue-i18n'
import SegmentItem from './SegmentItem.vue'

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
        <SegmentItem
          v-for="s in unreadSegments"
          :key="s.id"
          :segment="s"
          :expanded="isExpanded(s.id)"
          :unread="true"
          @toggle="toggleSegment(s)"
        />
      </ul>
    </section>

    <!-- 已读分组 -->
    <section v-if="readSegments.length" class="card">
      <button class="ghost section-toggle" @click="groupCollapsed.read = !groupCollapsed.read">
        {{ groupCollapsed.read ? '▶' : '▼' }} {{ t('receive.read') }} ({{ readSegments.length }})
      </button>
      <ul v-show="!groupCollapsed.read" class="seg-list">
        <SegmentItem
          v-for="s in readSegments"
          :key="s.id"
          :segment="s"
          :expanded="isExpanded(s.id)"
          :unread="false"
          @toggle="toggleSegment(s)"
        />
      </ul>
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

textarea {
  resize: vertical;
}
</style>
