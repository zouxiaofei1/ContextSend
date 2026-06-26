import { defineStore } from 'pinia'
import { ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { IPC, EVENT } from '../constants'
import { translate as t } from '../i18n'
import { useToastStore } from './toast'
import { useSettingsStore } from './settings'
import { usePermissions } from './modules/usePermissions'
import { useSegments } from './modules/useSegments'
import { useDevices } from './modules/useDevices'
import { usePairing } from './modules/usePairing'
import { useAdapters } from './modules/useAdapters'
import type { AppInfo, Device, NetEvent, SelfIdentity } from './types'

// 领域类型重新导出，组件仍可从 `stores/app` 导入（如 ConversationSegment）。
export type * from './types'

/**
 * 应用级 Pinia store（门面）：应用信息、本机身份与初始化 / 事件订阅编排。
 * 设备 / 配对 / 权限 / 对话段 / 适配器等职责拆到 `stores/modules/*`，
 * 在此组合为单一 store 实例，对外 API 与拆分前保持一致。
 */
export const useAppStore = defineStore('app', () => {
  const info = ref<AppInfo | null>(null)
  const identity = ref<SelfIdentity | null>(null)
  const loading = ref(false)

  /** 全局通知队列，用户可见的成功/错误提示统一走 toast。 */
  const toast = useToastStore()

  // ---- 职责模块（state 与 actions 在此组合为同一 store 实例） ----
  const permissions = usePermissions()
  const segments = useSegments()
  const devices = useDevices()
  const pairing = usePairing({
    permissionOf: permissions.permissionOf,
    setPermission: permissions.setPermission,
  })
  const adapters = useAdapters()

  /** 监听高级设置中的清理策略变更，实时生效。 */
  watch(
    () => {
      const s = useSettingsStore()
      return [s.conversationRetention, s.maxConversationCount] as const
    },
    () => {
      segments.enforceCleanup()
    },
  )

  /** 初始化：拉取应用信息、身份、设备列表，并订阅后端事件。 */
  async function init(): Promise<void> {
    loading.value = true
    try {
      // 先订阅事件，避免错过网络就绪后立即发现的设备；三个独立查询并行拉取。
      await subscribe()
      await segments.loadSegments()
      segments.enforceCleanup()
      await permissions.loadPermissions()
      const [appInfo, self, devs] = await Promise.all([
        invoke<AppInfo>(IPC.GET_APP_INFO),
        invoke<SelfIdentity>(IPC.GET_SELF_IDENTITY),
        invoke<Device[]>(IPC.LIST_DEVICES),
      ])
      info.value = appInfo
      identity.value = self
      devices.devices.value = devs
    } catch (e) {
      toast.error(String(e))
    } finally {
      loading.value = false
    }
  }

  /** 订阅后端 `net-event`，把设备/配对/接收事件分发给各模块；并在网络就绪后刷新设备列表。 */
  async function subscribe(): Promise<void> {
    // 网络服务后台异步启动，就绪后补拉一次权威设备快照（消除启动竞态）。
    await listen(EVENT.NET_READY, () => {
      void devices.refreshDevices()
    })
    await listen<NetEvent>(EVENT.NET_EVENT, (event) => {
      const p = event.payload
      switch (p.type) {
        case 'deviceFound':
          devices.upsertDevice({ id: p.id, name: p.name, online: p.online })
          break
        case 'deviceLost':
          devices.removeDevice(p.uuid)
          break
        case 'incomingPairing':
          pairing.handleIncomingPairing(p, permissions.permissionOf(p.peerUuid))
          break
        case 'conversationReceived':
          segments.addSegment(p.fromName, p.conversation, false)
          toast.info(
            t('receive.received', { name: p.fromName, count: p.conversation.messages.length }),
          )
          break
        case 'failed':
          toast.error(t('toast.pairingFailed', { reason: p.reason }))
          break
      }
    })
  }

  /** 给本机改名。 */
  async function renameSelf(name: string): Promise<void> {
    await invoke(IPC.RENAME_SELF, { newName: name })
    if (identity.value) identity.value.name = name
  }

  return {
    // 门面自有状态
    info,
    identity,
    loading,
    init,
    renameSelf,
    // 权限
    permissions: permissions.permissions,
    permissionOf: permissions.permissionOf,
    setPermission: permissions.setPermission,
    // 设备
    devices: devices.devices,
    // 配对
    incoming: pairing.incoming,
    outgoing: pairing.outgoing,
    startPairing: pairing.startPairing,
    confirmAndPush: pairing.confirmAndPush,
    acceptIncoming: pairing.acceptIncoming,
    rejectIncoming: pairing.rejectIncoming,
    // 对话段
    segments: segments.segments,
    selectedSegmentId: segments.selectedSegmentId,
    addSegment: segments.addSegment,
    markRead: segments.markRead,
    markAllRead: segments.markAllRead,
    removeSegment: segments.removeSegment,
    clearSegments: segments.clearSegments,
    selectSegment: segments.selectSegment,
    // 适配器
    importOpenai: adapters.importOpenai,
    exportOpenai: adapters.exportOpenai,
    matchContext: adapters.matchContext,
    importToApp: adapters.importToApp,
  }
})
