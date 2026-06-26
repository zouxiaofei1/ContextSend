<script setup lang="ts">
import { computed, ref } from 'vue'
import { useAppStore } from '../stores/app'
import type { Conversation, Device, PermissionLevel } from '../stores/app'
import { useI18n } from 'vue-i18n'

const app = useAppStore()
const { t, locale } = useI18n()

/** 推送源：接收页选定的段，回退到最新段，再回退到示例对话。 */
const pushConversation = computed<Conversation>(() => {
  const seg = app.segments.find((s) => s.id === app.selectedSegmentId) ?? app.segments[0]
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

/**
 * 设备排序：在线优先，离线沉底；同组内本机优先，再按名称稳定排序。
 * 离线设备不再消失，而是默认排在列表最下面。
 */
const sortedDevices = computed(() => {
  const selfId = app.identity?.uuid
  return [...app.devices].sort((a, b) => {
    if (a.online !== b.online) return a.online ? -1 : 1
    if (a.id === selfId) return -1
    if (b.id === selfId) return 1
    return a.name.localeCompare(b.name)
  })
})

const onlineCount = computed(() => app.devices.filter((d) => d.online).length)

/** 平台图标：按 os 标识映射到一个 emoji 字形（未知则用通用机器图标）。 */
function platformIcon(os?: string): string {
  switch (os) {
    case 'windows':
      return '🪟'
    case 'macos':
      return '🍎'
    case 'linux':
      return '🐧'
    default:
      return '💻'
  }
}

/**
 * 上次同步时间拆成 日期 / 时间 两段，并标记是否今天，供响应式按需隐藏。
 * 无记录返回 null（展示「从未同步」）。
 */
function syncParts(ts?: number): { date: string; time: string; isToday: boolean } | null {
  if (!ts) return null
  const loc = locale.value === 'zh-CN' ? 'zh-CN' : 'en-US'
  const d = new Date(ts)
  const now = new Date()
  const isToday =
    d.getFullYear() === now.getFullYear() &&
    d.getMonth() === now.getMonth() &&
    d.getDate() === now.getDate()
  const date = d.toLocaleDateString(loc, { month: 'short', day: 'numeric' })
  const time = d.toLocaleTimeString(loc, { hour: '2-digit', minute: '2-digit' })
  return { date, time, isToday }
}

/** 设备行 = 排序后的设备 + 预算好的同步时间分段（避免模板里重复调用）。 */
const deviceRows = computed(() =>
  sortedDevices.value.map((d) => ({ device: d, sync: syncParts(d.lastSync) })),
)

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

/** 右键上下文菜单：目标设备 + 屏幕坐标（null 表示未打开）。 */
const ctxMenu = ref<{ device: Device; x: number; y: number } | null>(null)

/** 在设备卡片上右键：本机不弹菜单；其余设备打开上下文菜单于光标处。 */
function openContextMenu(d: Device, e: MouseEvent): void {
  if (d.id === app.identity?.uuid) return
  openMenuId.value = null
  ctxMenu.value = { device: d, x: e.clientX, y: e.clientY }
}

function closeContextMenu(): void {
  ctxMenu.value = null
}

/** 上下文菜单「推送到设备」。 */
function ctxPush(): void {
  const d = ctxMenu.value?.device
  closeContextMenu()
  if (d) push(d)
}

/** 上下文菜单「忘记此设备」（仅离线时可见）。 */
function ctxForget(): void {
  const d = ctxMenu.value?.device
  closeContextMenu()
  if (d) app.forgetDevice(d.id)
}
</script>

<template>
  <div class="panel">
    <section class="card">
      <h2>{{ t('device.title') }} ({{ onlineCount }}/{{ app.devices.length }})</h2>
      <ul v-if="app.devices.length" class="device-list">
        <li
          v-for="{ device: d, sync } in deviceRows"
          :key="d.id"
          class="device-card"
          :class="{ offline: !d.online }"
          @contextmenu.prevent="openContextMenu(d, $event)"
        >
          <!-- L1：左 平台图标 + 名称；右 权限点 + 在线/离线 -->
          <div class="row row1">
            <span class="platform" :title="d.os || ''">{{ platformIcon(d.os) }}</span>
            <span class="device-name">{{ d.name }}</span>
            <span class="muted me" v-if="d.id === app.identity?.uuid">({{ t('device.me') }})</span>
            <span class="spacer" />
            <span
              v-if="d.id !== app.identity?.uuid"
              class="dot"
              :class="levelMeta(app.permissionOf(d.id)).cls"
              :title="t(`device.permission.${levelMeta(app.permissionOf(d.id)).key}`)"
            />
            <span v-else class="dot self" />
            <span class="status" :class="{ on: d.online }">
              {{ d.online ? t('device.statusOnline') : t('device.statusOffline') }}
            </span>
          </div>

          <!-- L2：左 IP；右 上次同步时间。窄屏按序隐藏：标签 →（今天隐日期 / 非今天隐时间） -->
          <div class="row row2 muted">
            <span class="ip">{{ d.ip || '—' }}</span>
            <span class="spacer" />
            <span v-if="sync" class="sync" :class="{ today: sync.isToday }">
              <span class="sync-label">{{ t('device.lastSync') }}: </span>
              <span class="sync-date">{{ sync.date }} </span>
              <span class="sync-time">{{ sync.time }}</span>
            </span>
            <span v-else class="sync">{{ t('device.neverSynced') }}</span>
          </div>

          <!-- L3：右 推送 + 权限按钮（本机不显示操作） -->
          <div v-if="d.id !== app.identity?.uuid" class="row row3">
            <span class="spacer" />
            <button class="small" :disabled="app.permissionOf(d.id) === -1" @click="push(d)">
              {{ t('device.push') }}
            </button>
            <div class="more">
              <button
                class="ghost small more-btn"
                :title="t('device.setPermission')"
                @click="toggleMenu(d.id)"
              >
                ⋯
              </button>
              <template v-if="openMenuId === d.id">
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
                    <span>{{ t(`device.permission.${m.key}`) }}</span>
                    <span v-if="app.permissionOf(d.id) === m.level" class="check">✓</span>
                  </button>
                </div>
              </template>
            </div>
          </div>
        </li>
      </ul>
      <p v-else class="muted">{{ t('device.noDevices') }}</p>
    </section>

    <!-- 设备右键上下文菜单：定位到光标处，点击外部关闭 -->
    <Teleport to="body">
      <div v-if="ctxMenu" class="menu-backdrop ctx-backdrop" @click="closeContextMenu" />
      <div
        v-if="ctxMenu"
        class="menu ctx-menu"
        role="menu"
        :style="{ left: ctxMenu.x + 'px', top: ctxMenu.y + 'px' }"
      >
        <button
          class="menu-item"
          :disabled="app.permissionOf(ctxMenu.device.id) === -1"
          @click="ctxPush"
        >
          {{ t('device.pushToDevice') }}
        </button>
        <button v-if="!ctxMenu.device.online" class="menu-item danger" @click="ctxForget">
          {{ t('device.forget') }}
        </button>
      </div>
    </Teleport>

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
  display: flex;
  flex-direction: column;
  gap: 0.6rem;
  /* 作为容器查询基准：卡片内文案按列表实际宽度（而非窗口宽度）逐级隐藏。 */
  container: devlist / inline-size;
}

/* ===== 三行设备卡片 ===== */
.device-card {
  display: flex;
  flex-direction: column;
  gap: 0.3rem;
  padding: 0.7rem 0.85rem;
  border: 1px solid var(--border);
  border-radius: 10px;
  background: var(--bg-primary);
  font-size: 0.9rem;
  min-width: 0;
}

.device-card.offline {
  opacity: 0.6;
}

.device-card .row {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  min-width: 0;
}

.device-card .row2 {
  font-size: 0.78rem;
}

.device-card .row3 {
  margin-top: 0.15rem;
}

.platform {
  font-size: 1.05rem;
  line-height: 1;
  flex: none;
}

.device-name {
  color: var(--text-primary);
  font-weight: 500;
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
  min-width: 0;
}

.me {
  flex: none;
  font-size: 0.8rem;
}

.ip {
  font-variant-numeric: tabular-nums;
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
  min-width: 0;
}

.sync {
  flex: none;
  white-space: nowrap;
}

/* 窄屏逐级隐藏：先去掉「上次同步」标签，再按今天/非今天去掉冗余的一段。 */
@container devlist (max-width: 340px) {
  .sync-label {
    display: none;
  }
}

@container devlist (max-width: 270px) {
  /* 今天：日期冗余，只留时间 */
  .sync.today .sync-date {
    display: none;
  }
  /* 非今天：时间次要，只留日期 */
  .sync:not(.today) .sync-time {
    display: none;
  }
}

/* 在线/离线文字标记 */
.status {
  flex: none;
  font-size: 0.75rem;
  color: var(--text-secondary);
}

.status.on {
  color: var(--success);
}

/* 弹性空隙：占满中间，把右侧内容推到行尾 */
.spacer {
  flex: 1;
}

/* ===== 圆点信任等级着色 ===== */
.dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex: none;
  background: var(--text-secondary);
}

.dot.self {
  background: var(--success);
}

.dot.blocked {
  background: var(--danger);
}

.dot.stranger {
  background: var(--warning);
}

.dot.trusted {
  background: var(--success);
}

.dot.sync {
  background: var(--accent);
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

.menu-item:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.menu-item.danger {
  color: var(--danger);
}

/* ===== 右键上下文菜单（teleport 到 body，定位到光标处） ===== */
.ctx-backdrop {
  z-index: 110;
}

.ctx-menu {
  position: fixed;
  right: auto;
  top: auto;
  z-index: 111;
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
