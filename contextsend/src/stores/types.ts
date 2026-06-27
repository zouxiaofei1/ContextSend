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
  buildNumber: number
}

/** 适配器可配置字段（与 Rust `AdapterField` 对齐，决定子页展示哪些项）。 */
export type AdapterField = 'dataDir' | 'installDir' | 'port'

/** 与 Rust 端 `cs_adapters::AdapterInfo` 对应：适配器探测状态 + 当前生效配置。 */
export interface AdapterInfo {
  name: string
  installed: boolean
  fields: AdapterField[]
  dataDir: string | null
  installDir: string | null
  port: number | null
}

/** 适配器配置覆盖（写入 `set_adapter_config`；字段省略 / null 表示沿用默认）。 */
export interface AdapterConfig {
  dataDir?: string | null
  installDir?: string | null
  port?: number | null
}

/** 本机身份。 */
export interface SelfIdentity {
  uuid: string
  name: string
}

/** 设备列表项，对应 Rust `Device`（外加前端持久化的 `lastSync`）。 */
export interface Device {
  id: string
  name: string
  online: boolean
  /** 操作系统标识（windows / linux / macos…），用于展示平台图标。后端发现事件携带。 */
  os?: string
  /** 用于展示的首选 IP 地址。后端发现事件携带。 */
  ip?: string
  /** 上次与该设备成功同步（推送/接收）的时间戳（毫秒）。纯前端持久化，不来自后端。 */
  lastSync?: number
}

/**
 * 设备权限等级（见路线图「权限模型」）。纯本地、非对称策略。
 * -1 已屏蔽 / 0 陌生人（默认）/ 1 已信任 / 2 自动同步（升级须过配对码）。
 */
export type PermissionLevel = -1 | 0 | 1 | 2

/** 一条消息的 token 用量明细（与 Rust `TokenUsage` 对齐，字段均可选）。 */
export interface TokenUsage {
  inputTokens?: number
  outputTokens?: number
  totalTokens?: number
  reasoningTokens?: number
  cachedInputTokens?: number
}

/** 一条消息的生成元数据（与 Rust `MessageMetadata` 对齐，通常仅 assistant 消息携带）。 */
export interface MessageMetadata {
  model?: string
  provider?: string
  usage?: TokenUsage
  firstTokenLatencyMs?: number
  finishReason?: string
}

/** 一条对话消息（最小视图，多模态时 content 可能是数组）。 */
export interface ChatMessage {
  role: string
  content: unknown
  name?: string
  /** 生成元数据（模型 / token 用量 / 首字延迟）；由支持的适配器读取时填充。 */
  metadata?: MessageMetadata
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
  | { type: 'deviceFound'; id: string; name: string; online: boolean; os: string; ip: string }
  | { type: 'deviceLost'; uuid: string }
  | { type: 'incomingPairing'; pairingId: number; peerUuid: string; peerName: string; pin: string }
  | {
      type: 'conversationReceived'
      fromUuid: string
      fromName: string
      conversation: Conversation
    }
  | { type: 'failed'; pairingId: number; reason: string }
