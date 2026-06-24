<script setup lang="ts">
import { computed } from 'vue'
import { useAppStore } from '../stores/app'
import type { Conversation } from '../stores/app'
import { useI18n } from 'vue-i18n'

const app = useAppStore()
const { t } = useI18n()

/** 推送源：接收页选定的段，回退到最新段，再回退到示例对话。 */
const pushConversation = computed<Conversation>(() => {
  const seg =
    app.segments.find((s) => s.id === app.selectedSegmentId) ?? app.segments[0]
  if (seg) return seg.conversation
  return {
    title: '示例对话',
    model: 'gpt-4o',
    messages: [
      { role: 'system', content: '你是一个有用的助手。' },
      { role: 'user', content: '你好，帮我介绍下自己。' },
      { role: 'assistant', content: '我是本地 Chat AI 助手。' },
    ],
  }
})

const onlineDevices = computed(() => app.devices.filter((d) => d.online))
const offlineDevices = computed(() => app.devices.filter((d) => !d.online))
</script>

<template>
  <div class="panel">
    <!-- 错误/状态 -->
    <div v-if="app.error" class="banner banner--error">{{ app.error }}</div>
    <div v-if="app.status" class="banner banner--status">{{ app.status }}</div>

    <!-- 主动配对：显示配对码 -->
    <section v-if="app.outgoing" class="card card--accent">
      <h2>{{ t('device.pairingTitle') }}</h2>
      <p class="pin">{{ app.outgoing.pin }}</p>
      <p class="muted">{{ t('device.pairingHint') }}</p>
      <div class="row">
        <button @click="app.confirmAndPush(pushConversation)">
          {{ t('device.pairingConfirm') }}
        </button>
        <button class="ghost" @click="app.outgoing = null">{{ t('device.cancel') }}</button>
      </div>
    </section>

    <!-- 入站配对 -->
    <section v-if="app.incoming" class="card card--accent">
      <h2>{{ t('device.fromRequest', { name: app.incoming.peerName }) }}</h2>
      <p class="pin">{{ app.incoming.pin }}</p>
      <div class="row">
        <button @click="app.acceptIncoming()">{{ t('device.accept') }}</button>
        <button class="ghost" @click="app.rejectIncoming()">{{ t('device.reject') }}</button>
      </div>
    </section>

    <!-- 在线设备 -->
    <section class="card">
      <h2>{{ t('device.online') }} ({{ onlineDevices.length }})</h2>
      <ul v-if="onlineDevices.length" class="device-list">
        <li v-for="d in onlineDevices" :key="d.id" class="device-item">
          <div class="device-item__info">
            <span class="dot online" />
            <span class="device-name">{{ d.name }}</span>
            <span class="muted" v-if="d.id === app.identity?.uuid">({{ t('device.me') }})</span>
            <span class="permission-badge stranger">{{ t('device.permission.stranger') }}</span>
          </div>
          <div class="device-item__actions">
            <button
              v-if="d.id !== app.identity?.uuid"
              class="small"
              @click="app.startPairing(d.id)"
            >
              {{ t('device.pair') }}
            </button>
          </div>
        </li>
      </ul>
      <p v-if="app.devices.length === 0" class="muted">{{ t('device.noDevices') }}</p>
    </section>

    <!-- 离线设备 -->
    <section v-if="offlineDevices.length" class="card">
      <h2>{{ t('device.offline') }} ({{ offlineDevices.length }})</h2>
      <ul class="device-list">
        <li v-for="d in offlineDevices" :key="d.id" class="device-item">
          <div class="device-item__info">
            <span class="dot" />
            <span class="device-name">{{ d.name }}</span>
            <span class="muted">— {{ t('device.neverConnected') }}</span>
          </div>
        </li>
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

.card h2 {
  margin: 0 0 0.5rem;
  font-size: 1.05rem;
}

.muted {
  color: var(--text-secondary);
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

.device-list {
  margin: 0;
  padding-left: 0;
  list-style: none;
}

.device-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0.4rem 0;
  border-bottom: 1px solid var(--border);
}

.device-item:last-child {
  border-bottom: none;
}

.device-item__info {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  font-size: 0.9rem;
}

.device-name {
  color: var(--text-primary);
  font-weight: 500;
}

.device-item__actions {
  flex-shrink: 0;
}
</style>
