import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

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

/** 待用户确认的入站配对。 */
export interface IncomingPairing {
  pairingId: number
  peerUuid: string
  peerName: string
  pin: string
}

/** 主动发起配对后等待确认的状态。 */
export interface OutgoingPairing {
  pairingId: number
  targetUuid: string
  pin: string
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
  const received = ref<{ fromName: string; conversation: Conversation } | null>(null)
  const status = ref<string>('')
  const error = ref<string | null>(null)
  const loading = ref(false)

  /** 初始化：拉取应用信息、身份、设备列表，并订阅后端事件。 */
  async function init(): Promise<void> {
    loading.value = true
    error.value = null
    try {
      // 先订阅事件，避免错过网络就绪后立即发现的设备；三个独立查询并行拉取。
      await subscribe()
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
        case 'incomingPairing':
          incoming.value = {
            pairingId: p.pairingId,
            peerUuid: p.peerUuid,
            peerName: p.peerName,
            pin: p.pin,
          }
          break
        case 'conversationReceived':
          received.value = { fromName: p.fromName, conversation: p.conversation }
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

  /** 主动向设备发起配对，进入「等待比对配对码」状态。 */
  async function startPairing(targetUuid: string): Promise<void> {
    error.value = null
    const res = await invoke<{ pairingId: number; pin: string }>('connect_pair', {
      targetUuid,
    })
    outgoing.value = { pairingId: res.pairingId, targetUuid, pin: res.pin }
  }

  /** 确认主动配对码一致后，推送给定对话。 */
  async function confirmAndPush(conversation: Conversation): Promise<void> {
    if (!outgoing.value) return
    await invoke('push_conversation', {
      pairingId: outgoing.value.pairingId,
      conversation,
    })
    status.value = '已推送当前对话'
    outgoing.value = null
  }

  /** 确认入站配对码一致后，接收对端对话。 */
  async function acceptIncoming(): Promise<void> {
    if (!incoming.value) return
    await invoke('accept_incoming', { pairingId: incoming.value.pairingId })
    incoming.value = null
  }

  /** 拒绝当前入站配对。 */
  async function rejectIncoming(): Promise<void> {
    if (!incoming.value) return
    await invoke('reject_pairing', { pairingId: incoming.value.pairingId })
    incoming.value = null
  }

  /** 导入 OpenAI Compatible JSON 文本。 */
  async function importOpenai(json: string): Promise<Conversation> {
    return await invoke<Conversation>('import_openai', { json })
  }

  /** 导出对话为 OpenAI Compatible JSON 文本。 */
  async function exportOpenai(conversation: Conversation): Promise<string> {
    return await invoke<string>('export_openai', { conversation })
  }

  return {
    info,
    identity,
    devices,
    incoming,
    outgoing,
    received,
    status,
    error,
    loading,
    init,
    renameSelf,
    startPairing,
    confirmAndPush,
    acceptIncoming,
    rejectIncoming,
    importOpenai,
    exportOpenai,
  }
})
