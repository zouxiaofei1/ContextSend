/**
 * store 层共享的领域类型。从原 `stores/app.ts` 抽出，供门面 store 与各
 * `stores/modules/*` 模块共享，避免循环依赖。门面 `stores/app.ts` 会
 * `export *` 这些类型，组件仍可从 `stores/app` 导入。
 */

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
export type NetEvent =
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
