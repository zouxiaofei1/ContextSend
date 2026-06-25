import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { load, type Store } from '@tauri-apps/plugin-store'

/** 与 Rust 端 `commands::AppInfo` 对应的应用信息。 */
export interface AppInfo {
  version: string
  platform: string
  adapters: string[]
}

/** 本机身份。 */
export interface SelfIdentity {
  uuid: string
  name: string
}

/** 设备列表项，对应 Rust `Device`。 */
export interface Device {
  id: string
  name: string
  online: boolean
}

/**
 * 设备权限等级（见路线图「权限模型」）。纯本地、非对称策略。
 * -1 已屏蔽 / 0 陌生人（默认）/ 1 已信任 / 2 自动同步（升级须过配对码）。
 */
export type PermissionLevel = -1 | 0 | 1 | 2

/** 一条对话消息（最小视图，多模态时 content 可能是数组）。 */
export interface ChatMessage {
  role: string
  content: unknown
  name?: string
}

/** 一段对话，对应 Rust `Conversation`。 */
export interface Conversation {
  title?: string
  model?: string
  messages: ChatMessage[]
}

/**
 * 接收页中的一段对话上下文：包裹一份 {@link Conversation}，附带来源、时间与已读状态。
 * 多段对话以 {@link ConversationSegment} 列表形式保存并持久化到磁盘。
 */
export interface ConversationSegment {
  id: string
  /** 来源设备名；本地导入用 i18n 文案占位。 */
  fromName: string
  /** 接收/导入时间戳（毫秒）。 */
  receivedAt: number
  /** 已读 / 未读 —— 接收页分组与折叠的依据。 */
  read: boolean
  conversation: Conversation
}

/** 待用户确认的入站配对。 */
export interface IncomingPairing {
  pairingId: number
  peerUuid: string
  peerName: string
  pin: string
  /** 是否需要展示并比对 6 位配对码（仅 Level 2 升级流程为 true）。 */
  showPin: boolean
}

/** 主动发起配对后等待确认的状态。 */
export interface OutgoingPairing {
  pairingId: number
  targetUuid: string
  pin: string
  /** 该次配对是否为「升级到 Level 2」流程；确认后写入 Level 2。 */
  upgrade: boolean
  /** 是否需要展示并比对 6 位配对码（仅升级流程为 true）。 */
  showPin: boolean
  /** 确认后要推送的对话。 */
  conversation: Conversation
}

/** 后端 `net-event` 事件（与 Rust `NetEvent` 的 serde 标签对应）。 */
type NetEvent =
  | { type: 'deviceFound'; id: string; name: string; online: boolean }
  | { type: 'deviceLost'; uuid: string }
  | { type: 'incomingPairing'; pairingId: number; peerUuid: string; peerName: string; pin: string }
  | {
      type: 'conversationReceived'
      fromUuid: string
      fromName: string
      conversation: Conversation
    }
  | { type: 'failed'; pairingId: number; reason: string }

/**
 * 应用级 Pinia store：应用信息、本机身份、设备列表、配对状态、收到的对话。
 */
export const useAppStore = defineStore('app', () => {
  const info = ref<AppInfo | null>(null)
  const identity = ref<SelfIdentity | null>(null)
  const devices = ref<Device[]>([])
  const incoming = ref<IncomingPairing | null>(null)
  const outgoing = ref<OutgoingPairing | null>(null)
  /** 已接收 / 导入的多段对话（最新在前），持久化到磁盘。 */
  const segments = ref<ConversationSegment[]>([])
  /** 配对推送时选用的段 id（默认最新段）。 */
  const selectedSegmentId = ref<string | null>(null)
  const status = ref<string>('')
  const error = ref<string | null>(null)
  const loading = ref(false)

  /** 各设备权限等级（本地、非对称），按 device uuid 索引；未记录的默认 Level 0。 */
  const permissions = ref<Record<string, PermissionLevel>>({})

  // ---- 持久化（Tauri plugin-store，磁盘 JSON） ----
  const STORE_FILE = 'segments.json'
  const STORE_KEY = 'segments'
  let store: Store | null = null

  // 权限单独存一份磁盘 JSON（纯本地策略，按设备 uuid 记录等级）。
  const PERM_FILE = 'permissions.json'
  const PERM_KEY = 'permissions'
  let permStore: Store | null = null

  /** 简易 UUID（优先 crypto.randomUUID，回退到时间戳+随机）。 */
  function newId(): string {
    if (typeof crypto !== 'undefined' && 'randomUUID' in crypto) {
      return crypto.randomUUID()
    }
    return `${Date.now()}-${Math.random().toString(16).slice(2)}`
  }

  /** 把当前 segments 落盘（异步，失败仅记录不阻断 UI）。 */
  async function persistSegments(): Promise<void> {
    try {
      if (!store) store = await load(STORE_FILE, { defaults: {}, autoSave: false })
      await store.set(STORE_KEY, segments.value)
      await store.save()
    } catch (e) {
      console.error('持久化对话失败:', e)
    }
  }

  /** 从磁盘恢复 segments。 */
  async function loadSegments(): Promise<void> {
    try {
      if (!store) store = await load(STORE_FILE, { defaults: {}, autoSave: false })
      const saved = await store.get<ConversationSegment[]>(STORE_KEY)
      if (Array.isArray(saved)) {
        segments.value = saved
        selectedSegmentId.value = saved[0]?.id ?? null
      }
    } catch (e) {
      console.error('恢复对话失败:', e)
    }
  }

  /** 把当前权限表落盘（失败仅记录不阻断 UI）。 */
  async function persistPermissions(): Promise<void> {
    try {
      if (!permStore) permStore = await load(PERM_FILE, { defaults: {}, autoSave: false })
      await permStore.set(PERM_KEY, permissions.value)
      await permStore.save()
    } catch (e) {
      console.error('持久化权限失败:', e)
    }
  }

  /** 从磁盘恢复权限表。 */
  async function loadPermissions(): Promise<void> {
    try {
      if (!permStore) permStore = await load(PERM_FILE, { defaults: {}, autoSave: false })
      const saved = await permStore.get<Record<string, PermissionLevel>>(PERM_KEY)
      if (saved && typeof saved === 'object') permissions.value = saved
    } catch (e) {
      console.error('恢复权限失败:', e)
    }
  }

  /** 读取某设备的权限等级（未记录则为默认 Level 0 陌生人）。 */
  function permissionOf(id: string): PermissionLevel {
    return permissions.value[id] ?? 0
  }

  /** 设置某设备的权限等级并落盘。Level 2 的升级须先经配对码验证（见 {@link startPairing}）。 */
  function setPermission(id: string, level: PermissionLevel): void {
    permissions.value[id] = level
    void persistPermissions()
  }

  /** 初始化：拉取应用信息、身份、设备列表，并订阅后端事件。 */
  async function init(): Promise<void> {
    loading.value = true
    error.value = null
    try {
      // 先订阅事件，避免错过网络就绪后立即发现的设备；三个独立查询并行拉取。
      await subscribe()
      await loadSegments()
      await loadPermissions()
      const [appInfo, self, devs] = await Promise.all([
        invoke<AppInfo>('get_app_info'),
        invoke<SelfIdentity>('get_self_identity'),
        invoke<Device[]>('list_devices'),
      ])
      info.value = appInfo
      identity.value = self
      devices.value = devs
    } catch (e) {
      error.value = String(e)
    } finally {
      loading.value = false
    }
  }

  /** 重新拉取设备列表快照（权威覆盖）。 */
  async function refreshDevices(): Promise<void> {
    try {
      devices.value = await invoke<Device[]>('list_devices')
    } catch {
      /* 网络尚未就绪时忽略 */
    }
  }

  /** 订阅后端 `net-event`，把设备/配对/接收事件落到 store；并在网络就绪后刷新设备列表。 */
  async function subscribe(): Promise<void> {
    // 网络服务后台异步启动，就绪后补拉一次权威设备快照（消除启动竞态）。
    await listen('net-ready', () => {
      void refreshDevices()
    })
    await listen<NetEvent>('net-event', (event) => {
      const p = event.payload
      switch (p.type) {
        case 'deviceFound': {
          const idx = devices.value.findIndex((d) => d.id === p.id)
          const dev: Device = { id: p.id, name: p.name, online: p.online }
          if (idx >= 0) devices.value[idx] = dev
          else devices.value.push(dev)
          break
        }
        case 'deviceLost':
          devices.value = devices.value.filter((d) => d.id !== p.uuid)
          break
        case 'incomingPairing': {
          // 按本机对该设备的权限等级（本地、非对称）决定如何处理入站推送。
          const level = permissionOf(p.peerUuid)
          if (level === -1) {
            // 已屏蔽：静默拒绝，不打扰用户。
            void invoke('reject_pairing', { pairingId: p.pairingId })
            break
          }
          if (level === 1) {
            // 已信任：自动接收，不弹窗、不比对 PIN。
            void invoke('accept_incoming', { pairingId: p.pairingId }).catch((e) => {
              error.value = `接收失败：${String(e)}`
            })
            break
          }
          // Level 0 陌生人：按设备名确认（不展示 PIN）；Level 2：展示 PIN 比对。
          incoming.value = {
            pairingId: p.pairingId,
            peerUuid: p.peerUuid,
            peerName: p.peerName,
            pin: p.pin,
            showPin: level >= 2,
          }
          break
        }
        case 'conversationReceived':
          addSegment(p.fromName, p.conversation, false)
          status.value = `收到来自「${p.fromName}」的 ${p.conversation.messages.length} 条消息`
          break
        case 'failed':
          error.value = `配对失败：${p.reason}`
          break
      }
    })
  }

  /** 给本机改名。 */
  async function renameSelf(name: string): Promise<void> {
    await invoke('rename_self', { newName: name })
    if (identity.value) identity.value.name = name
  }

  /**
   * 主动向设备推送对话。按本机对该设备的权限等级决定流程：
   * - Level 1（已信任）：建立加密会话后直接推送，不弹窗、不展示 PIN；
   * - Level 0（陌生人）：弹「发送给 X?」按名确认（不展示 PIN），确认后推送；
   * - upgrade=true（升级到 Level 2）：展示 6 位配对码比对，确认后推送并写入 Level 2。
   *
   * 后台无论哪一级都走完整加密握手，区别只在是否展示/比对配对码。
   */
  async function startPairing(
    targetUuid: string,
    conversation: Conversation,
    upgrade = false,
  ): Promise<void> {
    error.value = null
    try {
      const res = await invoke<{ pairingId: number; pin: string }>('connect_pair', {
        targetUuid,
      })
      // 已信任设备的普通推送：无需确认，直接推送（不弹窗、不展示 PIN）。
      if (!upgrade && permissionOf(targetUuid) === 1) {
        await invoke('push_conversation', { pairingId: res.pairingId, conversation })
        status.value = '已推送当前对话'
        return
      }
      // Level 0 陌生人（按名确认）或升级到 Level 2（比对 PIN）：弹窗等用户确认。
      outgoing.value = {
        pairingId: res.pairingId,
        targetUuid,
        pin: res.pin,
        upgrade,
        showPin: upgrade,
        conversation,
      }
    } catch (e) {
      error.value = `配对失败：${String(e)}`
    }
  }

  /** 确认后推送 `outgoing` 中暂存的对话；若为升级流程则把对端置为 Level 2。 */
  async function confirmAndPush(): Promise<void> {
    if (!outgoing.value) return
    const { pairingId, targetUuid, upgrade, conversation } = outgoing.value
    try {
      await invoke('push_conversation', { pairingId, conversation })
      if (upgrade) setPermission(targetUuid, 2)
      status.value = '已推送当前对话'
    } catch (e) {
      error.value = `推送失败：${String(e)}`
    } finally {
      outgoing.value = null
    }
  }

  /** 确认入站配对码一致后，接收对端对话。 */
  async function acceptIncoming(): Promise<void> {
    if (!incoming.value) return
    const { pairingId } = incoming.value
    incoming.value = null // 先关弹窗，避免请求失败时卡住遮罩。
    try {
      await invoke('accept_incoming', { pairingId })
    } catch (e) {
      error.value = `接收失败：${String(e)}`
    }
  }

  /** 拒绝当前入站配对。 */
  async function rejectIncoming(): Promise<void> {
    if (!incoming.value) return
    const { pairingId } = incoming.value
    incoming.value = null
    try {
      await invoke('reject_pairing', { pairingId })
    } catch (e) {
      error.value = `拒绝失败：${String(e)}`
    }
  }

  /** 导入 OpenAI Compatible JSON 文本。 */
  async function importOpenai(json: string): Promise<Conversation> {
    return await invoke<Conversation>('import_openai', { json })
  }

  /** 导出对话为 OpenAI Compatible JSON 文本。 */
  async function exportOpenai(conversation: Conversation): Promise<string> {
    return await invoke<string>('export_openai', { conversation })
  }

  // ---- 段管理 ----

  /** 新增一段对话（最新在前），并落盘。返回新段 id。 */
  function addSegment(fromName: string, conversation: Conversation, read: boolean): string {
    const seg: ConversationSegment = {
      id: newId(),
      fromName,
      receivedAt: Date.now(),
      read,
      conversation,
    }
    segments.value.unshift(seg)
    if (!selectedSegmentId.value) selectedSegmentId.value = seg.id
    void persistSegments()
    return seg.id
  }

  /** 标记某段为已读。 */
  function markRead(id: string): void {
    const seg = segments.value.find((s) => s.id === id)
    if (seg && !seg.read) {
      seg.read = true
      void persistSegments()
    }
  }

  /** 全部标记为已读。 */
  function markAllRead(): void {
    let changed = false
    for (const s of segments.value) {
      if (!s.read) {
        s.read = true
        changed = true
      }
    }
    if (changed) void persistSegments()
  }

  /** 删除一段对话。 */
  function removeSegment(id: string): void {
    segments.value = segments.value.filter((s) => s.id !== id)
    if (selectedSegmentId.value === id) {
      selectedSegmentId.value = segments.value[0]?.id ?? null
    }
    void persistSegments()
  }

  /** 清空所有段。 */
  function clearSegments(): void {
    segments.value = []
    selectedSegmentId.value = null
    void persistSegments()
  }

  /** 选定用于推送的段。 */
  function selectSegment(id: string): void {
    selectedSegmentId.value = id
  }

  return {
    info,
    identity,
    devices,
    incoming,
    outgoing,
    segments,
    selectedSegmentId,
    status,
    error,
    loading,
    permissions,
    init,
    renameSelf,
    startPairing,
    confirmAndPush,
    acceptIncoming,
    rejectIncoming,
    importOpenai,
    exportOpenai,
    permissionOf,
    setPermission,
    addSegment,
    markRead,
    markAllRead,
    removeSegment,
    clearSegments,
    selectSegment,
  }
})
