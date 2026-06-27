<script setup lang="ts">
import { ref } from 'vue'
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
/** 导入/导出区域是否展开 */
const showImportExport = ref(false)

function toggleSegment(s: ConversationSegment): void {
  if (expandedSegments.value.has(s.id)) {
    expandedSegments.value.delete(s.id)
  } else {
    expandedSegments.value.add(s.id)
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
        <button class="small ghost" @click="onClear()">{{ t('receive.clear') }}</button>
      </div>
    </div>

    <p v-if="app.segments.length === 0" class="muted empty">{{ t('receive.emptySegments') }}</p>

  
    <!-- 所有对话平铺列表 -->
    <section v-if="app.segments.length" class="card">
      <ul class="seg-list">
        <SegmentItem
          v-for="s in app.segments"
          :key="s.id"
          :segment="s"
          :expanded="isExpanded(s.id)"
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
