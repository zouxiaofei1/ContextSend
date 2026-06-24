<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useAppStore, type Conversation } from './stores/app'

const app = useAppStore()

/** 当前要推送的对话（来自导入或一段示例）。 */
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
const renameText = ref('')

onMounted(() => {
  void app.init()
})

async function onImport() {
  try {
    currentConversation.value = await app.importOpenai(importText.value)
    app.status = `已导入 ${currentConversation.value.messages.length} 条消息`
  } catch (e) {
    app.error = String(e)
  }
}

async function onExport() {
  try {
    exportText.value = await app.exportOpenai(currentConversation.value)
  } catch (e) {
    app.error = String(e)
  }
}

async function onRename() {
  if (renameText.value.trim()) {
    await app.renameSelf(renameText.value.trim())
    renameText.value = ''
  }
}

/** 文本块预览（多模态时退化为占位）。 */
function preview(content: unknown): string {
  if (typeof content === 'string') return content
  return '[多模态内容]'
}
</script>

<template>
  <main class="app">
    <header class="app__header">
      <h1>ContextSend</h1>
      <p class="app__subtitle">局域网内分享本地 Chat AI 上下文</p>
    </header>

    <p v-if="app.error" class="error">{{ app.error }}</p>
    <p v-if="app.status" class="status">{{ app.status }}</p>

    <section class="card">
      <h2>本机</h2>
      <p v-if="app.identity">
        名称：<strong>{{ app.identity.name }}</strong>
        <span class="muted"> （{{ app.identity.uuid.slice(0, 8) }}）</span>
      </p>
      <div class="row">
        <input v-model="renameText" placeholder="改名…" />
        <button @click="onRename">改名</button>
      </div>
    </section>

    <section class="card">
      <h2>设备列表</h2>
      <p v-if="app.devices.length === 0" class="muted">
        尚未发现设备（确保同一局域网，等待几秒）。
      </p>
      <ul v-else class="devices">
        <li v-for="d in app.devices" :key="d.id">
          <span class="dot" :class="{ online: d.online }"></span>
          {{ d.name }}
          <button @click="app.startPairing(d.id)">配对并推送</button>
        </li>
      </ul>
    </section>

    <!-- 主动配对：显示配对码，等用户比对一致后推送 -->
    <section v-if="app.outgoing" class="card card--accent">
      <h2>配对码（请与对方核对）</h2>
      <p class="pin">{{ app.outgoing.pin }}</p>
      <p class="muted">两端配对码一致才可推送，可防止中间人。</p>
      <div class="row">
        <button @click="app.confirmAndPush(currentConversation)">一致，推送当前对话</button>
        <button class="ghost" @click="app.outgoing = null">取消</button>
      </div>
    </section>

    <!-- 入站配对：对方请求，显示配对码 -->
    <section v-if="app.incoming" class="card card--accent">
      <h2>来自「{{ app.incoming.peerName }}」的配对请求</h2>
      <p class="pin">{{ app.incoming.pin }}</p>
      <div class="row">
        <button @click="app.acceptIncoming()">一致，接收</button>
        <button class="ghost" @click="app.rejectIncoming()">拒绝</button>
      </div>
    </section>

    <section class="card">
      <h2>当前对话（{{ currentConversation.messages.length }} 条）</h2>
      <ul class="messages">
        <li v-for="(m, i) in currentConversation.messages" :key="i">
          <b>{{ m.role }}：</b>{{ preview(m.content) }}
        </li>
      </ul>
    </section>

    <section class="card">
      <h2>导入 / 导出（OpenAI Compatible JSON）</h2>
      <textarea v-model="importText" rows="4" placeholder="粘贴 OpenAI JSON 后点击导入…"></textarea>
      <div class="row">
        <button @click="onImport">导入</button>
        <button @click="onExport">导出当前对话</button>
      </div>
      <textarea v-if="exportText" :value="exportText" rows="6" readonly></textarea>
    </section>
  </main>
</template>

<style scoped>
.app {
  max-width: 720px;
  margin: 0 auto;
  padding: 2rem 1.5rem;
}
.app__header h1 {
  margin: 0;
  font-size: 1.8rem;
}
.app__subtitle {
  margin: 0.25rem 0 1.5rem;
  color: #9aa0aa;
}
.card {
  background: #24272e;
  border: 1px solid #30343c;
  border-radius: 10px;
  padding: 1rem 1.25rem;
  margin-bottom: 1rem;
}
.card--accent {
  border-color: #4c7cf3;
}
.card h2 {
  margin: 0 0 0.5rem;
  font-size: 1.05rem;
}
.muted {
  color: #9aa0aa;
}
.error {
  color: #ff7a7a;
}
.status {
  color: #7ad88a;
}
.pin {
  font-size: 2rem;
  letter-spacing: 0.4rem;
  font-weight: 700;
  margin: 0.25rem 0;
}
.row {
  display: flex;
  gap: 0.5rem;
  margin-top: 0.5rem;
  flex-wrap: wrap;
}
.devices,
.messages {
  margin: 0;
  padding-left: 0;
  list-style: none;
}
.devices li {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.35rem 0;
}
.messages li {
  padding: 0.2rem 0;
  border-bottom: 1px solid #2c2f37;
}
.dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: #666;
  display: inline-block;
}
.dot.online {
  background: #7ad88a;
}
input,
textarea {
  width: 100%;
  background: #1c1f25;
  border: 1px solid #30343c;
  border-radius: 6px;
  color: #e6e6e6;
  padding: 0.4rem 0.6rem;
  font-family: inherit;
}
button {
  background: #4c7cf3;
  border: none;
  border-radius: 6px;
  color: #fff;
  padding: 0.4rem 0.8rem;
  cursor: pointer;
}
button.ghost {
  background: transparent;
  border: 1px solid #4c7cf3;
  color: #9bb6f7;
}
</style>
