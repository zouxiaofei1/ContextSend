<script setup lang="ts">
import { computed, ref } from 'vue'
import { useAppStore } from '../stores/app'
import type { Conversation, Device, PermissionLevel } from '../stores/app'
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

/** 权限等级元数据：UI 标签 i18n key + 徽章样式类。Level 2 切换需走配对流程。 */
const PERMISSION_LEVELS: { level: PermissionLevel; key: string; cls: string }[] = [
  { level: -1, key: 'blocked', cls: 'blocked' },
  { level: 0, key: 'stranger', cls: 'stranger' },
  { level: 1, key: 'trusted', cls: 'trusted' },
  { level: 2, key: 'sync', cls: 'sync' },
]

function levelMeta(level: PermissionLevel) {
  return PERMISSION_LEVELS.find((m) => m.level === level) ?? PERMISSION_LEVELS[1]
}

/** 当前展开「...」菜单的设备 id（null 表示无）。 */
const openMenuId = ref<string | null>(null)

function toggleMenu(id: string): void {
  openMenuId.value = openMenuId.value === id ? null : id
}

/** 「推送」按钮：按本机对该设备的权限等级走不同流程（Level 0 按名确认 / Level 1 直推 / 见 store）。 */
function push(d: Device): void {
  if (app.permissionOf(d.id) === -1) return
  void app.startPairing(d.id, pushConversation.value, false)
}

/** 主动配对弹窗里的目标设备名（升级/陌生人发送时展示）。 */
const outgoingName = computed(
  () => app.devices.find((d) => d.id === app.outgoing?.targetUuid)?.name ?? '',
)

/** 在「...」菜单内选择权限等级。升级到 Level 2 必须过配对码验证（弹出推送）。 */
function chooseLevel(d: Device, level: PermissionLevel): void {
  openMenuId.value = null
  if (level === app.permissionOf(d.id)) return
  if (level === 2) {
    void app.startPairing(d.id, pushConversation.value, true)
  } else {
    app.setPermission(d.id, level)
  }
}
</script>

<template>
  <div class="panel">
    <!-- 错误/状态 -->
    <div v-if="app.error" class="banner banner--error">{{ app.error }}</div>
    <div v-if="app.status" class="banner banner--status">{{ app.status }}</div>

    <!-- 在线设备 -->
    <section class="card">
      <h2>{{ t('device.online') }} ({{ onlineDevices.length }})</h2>
      <ul v-if="onlineDevices.length" class="device-list">
        <li v-for="d in onlineDevices" :key="d.id" class="device-item">
          <span class="dot online" />
          <span class="device-name">{{ d.name }}</span>
          <span class="muted" v-if="d.id === app.identity?.uuid">({{ t('device.me') }})</span>
          <span
            v-else
            class="permission-badge"
            :class="levelMeta(app.permissionOf(d.id)).cls"
          >
            {{ t(`device.permission.${levelMeta(app.permissionOf(d.id)).key}`) }}
          </span>

          <!-- 弹性空隙：把操作推到右侧 -->
          <span class="spacer" />

          <template v-if="d.id !== app.identity?.uuid">
            <button
              class="small"
              :disabled="app.permissionOf(d.id) === -1"
              @click="push(d)"
            >
              {{ t('device.push') }}
            </button>

            <!-- 「...」更多菜单 -->
            <div class="more">
              <button class="ghost small more-btn" :title="t('device.more')" @click="toggleMenu(d.id)">
                ⋯
              </button>
              <template v-if="openMenuId === d.id">
                <!-- 透明遮罩：点击外部关闭菜单 -->
                <div class="menu-backdrop" @click="openMenuId = null" />
                <div class="menu" role="menu">
                  <p class="menu-title">{{ t('device.setPermission') }}</p>
                  <button
                    v-for="m in PERMISSION_LEVELS"
                    :key="m.level"
                    class="menu-item"
                    :class="{ active: app.permissionOf(d.id) === m.level }"
                    @click="chooseLevel(d, m.level)"
                  >
                    <span class="permission-badge" :class="m.cls">
                      {{ t(`device.permission.${m.key}`) }}
                    </span>
                    <span v-if="app.permissionOf(d.id) === m.level" class="check">✓</span>
                  </button>
                </div>
              </template>
            </div>
          </template>
        </li>
      </ul>
      <p v-if="app.devices.length === 0" class="muted">{{ t('device.noDevices') }}</p>
    </section>

    <!-- 离线设备 -->
    <section v-if="offlineDevices.length" class="card">
      <h2>{{ t('device.offline') }} ({{ offlineDevices.length }})</h2>
      <ul class="device-list">
        <li v-for="d in offlineDevices" :key="d.id" class="device-item">
          <span class="dot" />
          <span class="device-name">{{ d.name }}</span>
          <span class="muted">— {{ t('device.neverConnected') }}</span>
        </li>
      </ul>
    </section>

    <!-- 配对码 / 确认弹窗（居中 + 灰色遮罩）：主动配对与入站配对二选一 -->
    <Teleport to="body">
      <div v-if="app.outgoing || app.incoming" class="modal-overlay">
        <!-- 主动配对 -->
        <section v-if="app.outgoing" class="card card--accent modal-card">
          <!-- 升级到 Level 2：展示配对码比对 -->
          <template v-if="app.outgoing.showPin">
            <h2>{{ t('device.pairingTitle') }}</h2>
            <p class="pin">{{ app.outgoing.pin }}</p>
            <p class="muted">{{ t('device.pairingHint') }}</p>
            <div class="row">
              <button @click="app.confirmAndPush()">{{ t('device.pairingConfirm') }}</button>
              <button class="ghost" @click="app.outgoing = null">{{ t('device.cancel') }}</button>
            </div>
          </template>
          <!-- Level 0 陌生人：按设备名确认，不展示配对码 -->
          <template v-else>
            <h2>{{ t('device.sendConfirm', { name: outgoingName }) }}</h2>
            <p class="muted">{{ t('device.sendConfirmHint') }}</p>
            <div class="row">
              <button @click="app.confirmAndPush()">{{ t('device.sendConfirmBtn') }}</button>
              <button class="ghost" @click="app.outgoing = null">{{ t('device.cancel') }}</button>
            </div>
          </template>
        </section>

        <!-- 入站配对 -->
        <section v-else-if="app.incoming" class="card card--accent modal-card">
          <!-- Level 2：展示配对码比对 -->
          <template v-if="app.incoming.showPin">
            <h2>{{ t('device.fromRequest', { name: app.incoming.peerName }) }}</h2>
            <p class="pin">{{ app.incoming.pin }}</p>
            <div class="row">
              <button @click="app.acceptIncoming()">{{ t('device.accept') }}</button>
              <button class="ghost" @click="app.rejectIncoming()">{{ t('device.reject') }}</button>
            </div>
          </template>
          <!-- Level 0 陌生人：按设备名确认接收，不展示配对码 -->
          <template v-else>
            <h2>{{ t('device.receiveConfirm', { name: app.incoming.peerName }) }}</h2>
            <p class="muted">{{ t('device.receiveConfirmHint') }}</p>
            <div class="row">
              <button @click="app.acceptIncoming()">{{ t('device.receiveBtn') }}</button>
              <button class="ghost" @click="app.rejectIncoming()">{{ t('device.reject') }}</button>
            </div>
          </template>
        </section>
      </div>
    </Teleport>
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
  align-items: center;
  gap: 0.5rem;
  padding: 0.4rem 0;
  border-bottom: 1px solid var(--border);
  font-size: 0.9rem;
}

.device-item:last-child {
  border-bottom: none;
}

.device-name {
  color: var(--text-primary);
  font-weight: 500;
}

/* 弹性空隙：占满中间，把推送 / 更多推到右侧 */
.spacer {
  flex: 1;
}

/* ===== 权限徽章 ===== */
.permission-badge {
  font-size: 0.72rem;
  font-weight: 600;
  padding: 0.1rem 0.45rem;
  border-radius: 999px;
  border: 1px solid currentColor;
  white-space: nowrap;
}

.permission-badge.blocked {
  color: var(--danger, #e5534b);
}

.permission-badge.stranger {
  color: #d9a441;
}

.permission-badge.trusted {
  color: #41c171;
}

.permission-badge.sync {
  color: var(--accent);
}

/* ===== 「...」更多菜单 ===== */
.more {
  position: relative;
}

.more-btn {
  line-height: 1;
  padding: 0.1rem 0.45rem;
  font-size: 1rem;
}

.menu-backdrop {
  position: fixed;
  inset: 0;
  z-index: 20;
}

.menu {
  position: absolute;
  right: 0;
  top: calc(100% + 4px);
  z-index: 21;
  min-width: 9rem;
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 0.35rem;
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.35);
}

.menu-title {
  margin: 0.1rem 0.35rem 0.35rem;
  font-size: 0.72rem;
  color: var(--text-secondary);
}

.menu-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
  gap: 0.5rem;
  background: transparent;
  color: var(--text-primary);
  border-radius: 6px;
  padding: 0.35rem 0.4rem;
  font-size: 0.8rem;
}

.menu-item:hover {
  background: var(--bg-primary);
}

.menu-item.active {
  background: var(--bg-primary);
}

.menu-item .check {
  color: var(--accent);
  font-weight: 700;
}

/* ===== 居中配对弹窗 + 灰色遮罩 ===== */
.modal-overlay {
  position: fixed;
  inset: 0;
  z-index: 100;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0.5);
}

.modal-card {
  margin: 0;
  width: min(420px, 90vw);
  box-shadow: 0 12px 40px rgba(0, 0, 0, 0.45);
}
</style>
