<script setup lang="ts">
import { ref, computed } from 'vue'
import { useAppStore, type Conversation } from '../stores/app'
import { useI18n } from 'vue-i18n'

const app = useAppStore()
const { t } = useI18n()

const currentConversation = ref<Conversation>({
  title: '示例对话',
  model: 'gpt-4o',
  messages: [
    { role: 'system', content: '你是一个有用的助手。' },
    { role: 'user', content: '你好，帮我介绍下自己。' },
    { role: 'assistant', content: '我是本地 Chat AI 助手。' },
    { role: 'user', content: '今天天气怎么样？' },
    { role: 'assistant', content: '我无法获取实时天气，但可以帮你查询方法。' },
  ],
})

const importText = ref('')
const exportText = ref('')
const expandedMessages = ref<Set<number>>(new Set())

/** 导入/导出区域是否展开 */
const showImportExport = ref(false)

const messageCount = computed(() => currentConversation.value.messages.length)

function preview(content: unknown): string {
  if (typeof content === 'string') return content
  return '[多模态内容]'
}

function toggleMessage(idx: number): void {
  if (expandedMessages.value.has(idx)) {
    expandedMessages.value.delete(idx)
  } else {
    expandedMessages.value.add(idx)
  }
}

function isExpanded(idx: number): boolean {
  return expandedMessages.value.has(idx)
}

async function onImport(): Promise<void> {
  try {
    currentConversation.value = await app.importOpenai(importText.value)
    expandedMessages.value.clear()
    app.status = t('receive.importSuccess', { count: currentConversation.value.messages.length })
  } catch (e) {
    app.error = String(e)
  }
}

async function onExport(): Promise<void> {
  try {
    exportText.value = await app.exportOpenai(currentConversation.value)
    app.status = t('receive.exportReady')
  } catch (e) {
    app.error = String(e)
  }
}
</script>

<template>
  <div class="panel">
    <!-- 通知区 -->
    <div v-if="app.error" class="banner banner--error">{{ app.error }}</div>
    <div v-if="app.status" class="banner banner--status">{{ app.status }}</div>

    <!-- 接收到的对话通知 -->
    <div v-if="app.received" class="card card--accent">
      <h3>
        {{ t('receive.received', { name: app.received.fromName, count: app.received.conversation.messages.length }) }}
      </h3>
      <button class="small" @click="currentConversation = app.received!.conversation; app.received = null">
        {{ t('receive.title') }} →
      </button>
    </div>

    <!-- 当前对话 -->
    <section class="card">
      <div class="card__header">
        <h2>{{ t('receive.title') }}</h2>
        <span class="muted">{{ t('receive.count', { count: messageCount }) }}</span>
      </div>

      <ul v-if="messageCount > 0" class="msg-list">
        <li
          v-for="(m, i) in currentConversation.messages"
          :key="i"
          class="msg-item"
          @click="toggleMessage(i)"
        >
          <div class="msg-item__header">
            <b class="msg-role">{{ m.role }}</b>
            <span class="msg-toggle muted">{{ isExpanded(i) ? '▲' : '▼' }}</span>
          </div>
          <div class="msg-content" :class="{ 'msg-content--collapsed': !isExpanded(i) }">
            {{ preview(m.content) }}
          </div>
        </li>
      </ul>
      <p v-else class="muted">{{ t('receive.noMessages') }}</p>
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

.card {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 1rem 1.25rem;
  margin-bottom: 1rem;
}

.card--accent {
  border-color: var(--accent);
}

.card__header {
  display: flex;
  justify-content: space-between;
  align-items: baseline;
  margin-bottom: 0.5rem;
}

.card h2 {
  margin: 0;
  font-size: 1.05rem;
}

.card h3 {
  margin: 0 0 0.5rem;
  font-size: 0.95rem;
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

/* 消息列表 */
.msg-list {
  margin: 0;
  padding-left: 0;
  list-style: none;
}

.msg-item {
  padding: 0.35rem 0;
  border-bottom: 1px solid var(--border);
  cursor: pointer;
  transition: background 0.1s;
}

.msg-item:hover {
  background: var(--bg-tertiary);
}

.msg-item__header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.msg-role {
  font-size: 0.85rem;
  color: var(--accent);
  text-transform: uppercase;
}

.msg-toggle {
  font-size: 0.65rem;
}

.msg-content {
  font-size: 0.9rem;
  padding-top: 0.15rem;
  white-space: pre-wrap;
  word-break: break-word;
}

.msg-content--collapsed {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 100%;
}

textarea {
  resize: vertical;
}
</style>
