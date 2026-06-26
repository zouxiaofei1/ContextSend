import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { IPC } from '../../constants'
import { translate as t } from '../../i18n'
import { useToastStore } from '../toast'
import type { Conversation, IncomingPairing, OutgoingPairing, PermissionLevel } from '../types'

/** 入站配对原始负载（来自 `net-event` 的 `incomingPairing`）。 */
interface IncomingPairingPayload {
  pairingId: number
  peerUuid: string
  peerName: string
  pin: string
}

/**
 * 配对模块：入站 / 出站配对状态与确认流程。权限查询 / 写入由门面注入
 * （依赖 {@link usePermissions}），避免模块间直接耦合。
 */
export function usePairing(deps: {
  permissionOf: (id: string) => PermissionLevel
  setPermission: (id: string, level: PermissionLevel) => void
  /** 推送成功后记录与该设备的同步时间（毫秒）。 */
  recordSync: (id: string, ts: number) => void
}) {
  const toast = useToastStore()
  const incoming = ref<IncomingPairing | null>(null)
  const outgoing = ref<OutgoingPairing | null>(null)

  /**
   * 按本机对该设备的权限等级（本地、非对称）处理入站推送：
   * Level -1 静默拒绝；Level 1 自动接收；Level 0/2 弹窗（Level 2 展示 PIN 比对）。
   */
  function handleIncomingPairing(p: IncomingPairingPayload, level: PermissionLevel): void {
    if (level === -1) {
      // 已屏蔽：静默拒绝，不打扰用户。
      void invoke(IPC.REJECT_PAIRING, { pairingId: p.pairingId })
      return
    }
    if (level === 1) {
      // 已信任：自动接收，不弹窗、不比对 PIN。
      void invoke(IPC.ACCEPT_INCOMING, { pairingId: p.pairingId }).catch((e) => {
        toast.error(t('toast.receiveFailed', { error: String(e) }))
      })
      return
    }
    // Level 0 陌生人：按设备名确认（不展示 PIN）；Level 2：展示 PIN 比对。
    incoming.value = {
      pairingId: p.pairingId,
      peerUuid: p.peerUuid,
      peerName: p.peerName,
      pin: p.pin,
      showPin: level >= 2,
    }
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
    try {
      const res = await invoke<{ pairingId: number; pin: string }>(IPC.CONNECT_PAIR, {
        targetUuid,
      })
      // 已信任设备的普通推送：无需确认，直接推送（不弹窗、不展示 PIN）。
      if (!upgrade && deps.permissionOf(targetUuid) === 1) {
        await invoke(IPC.PUSH_CONVERSATION, { pairingId: res.pairingId, conversation })
        deps.recordSync(targetUuid, Date.now())
        toast.success(t('device.pushedSuccess'))
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
      toast.error(t('toast.pairingFailed', { reason: String(e) }))
    }
  }

  /** 确认后推送 `outgoing` 中暂存的对话；若为升级流程则把对端置为 Level 2。 */
  async function confirmAndPush(): Promise<void> {
    if (!outgoing.value) return
    const { pairingId, targetUuid, upgrade, conversation } = outgoing.value
    try {
      await invoke(IPC.PUSH_CONVERSATION, { pairingId, conversation })
      deps.recordSync(targetUuid, Date.now())
      if (upgrade) deps.setPermission(targetUuid, 2)
      toast.success(t('device.pushedSuccess'))
    } catch (e) {
      toast.error(t('toast.pushFailed', { error: String(e) }))
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
      await invoke(IPC.ACCEPT_INCOMING, { pairingId })
    } catch (e) {
      toast.error(t('toast.receiveFailed', { error: String(e) }))
    }
  }

  /** 拒绝当前入站配对。 */
  async function rejectIncoming(): Promise<void> {
    if (!incoming.value) return
    const { pairingId } = incoming.value
    incoming.value = null
    try {
      await invoke(IPC.REJECT_PAIRING, { pairingId })
    } catch (e) {
      toast.error(t('toast.rejectFailed', { error: String(e) }))
    }
  }

  return {
    incoming,
    outgoing,
    handleIncomingPairing,
    startPairing,
    confirmAndPush,
    acceptIncoming,
    rejectIncoming,
  }
}
