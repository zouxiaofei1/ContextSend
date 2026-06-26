import { ref } from 'vue'
import { STORE_FILE, STORE_KEY, retentionToMs } from '../../constants'
import { createPersistentStore } from '../../utils/tauriStore'
import { useSettingsStore } from '../settings'
import type { Conversation, ConversationSegment } from '../types'

/** 简易 UUID（优先 crypto.randomUUID，回退到时间戳+随机）。 */
function newId(): string {
  if (typeof crypto !== 'undefined' && 'randomUUID' in crypto) {
    return crypto.randomUUID()
  }
  return `${Date.now()}-${Math.random().toString(16).slice(2)}`
}

/**
 * 对话段模块：已接收 / 导入的多段对话（最新在前），持久化到磁盘
 * （`segments.json`），并按高级设置中的保存期限 / 最大条数执行清理。
 */
export function useSegments() {
  /** 已接收 / 导入的多段对话（最新在前），持久化到磁盘。 */
  const segments = ref<ConversationSegment[]>([])
  /** 配对推送时选用的段 id（默认最新段）。 */
  const selectedSegmentId = ref<string | null>(null)
  const store = createPersistentStore(STORE_FILE.SEGMENTS)

  /** 把当前 segments 落盘（fire-and-forget）。 */
  function persist(): void {
    void store.set(STORE_KEY.SEGMENTS, segments.value)
  }

  /** 从磁盘恢复 segments。 */
  async function loadSegments(): Promise<void> {
    const saved = await store.get<ConversationSegment[]>(STORE_KEY.SEGMENTS)
    if (Array.isArray(saved)) {
      segments.value = saved
      selectedSegmentId.value = saved[0]?.id ?? null
    }
  }

  /** 根据保存期限清理过期对话（静默，不提示）。 */
  function enforceRetentionPolicy(): void {
    const settings = useSettingsStore()
    const maxMs = retentionToMs(settings.conversationRetention)
    if (maxMs === null) return
    const cutoff = Date.now() - maxMs
    const before = segments.value.length
    segments.value = segments.value.filter((s) => s.receivedAt >= cutoff)
    if (segments.value.length !== before) persist()
  }

  /** 根据最大条数限制对话数量（保留最新），-1/0 表示不限。 */
  function enforceMaxCount(): void {
    const settings = useSettingsStore()
    const max = settings.maxConversationCount
    if (max <= 0) return
    if (segments.value.length > max) {
      segments.value.sort((a, b) => b.receivedAt - a.receivedAt)
      segments.value = segments.value.slice(0, max)
      persist()
    }
  }

  /** 同时执行两项清理策略。 */
  function enforceCleanup(): void {
    enforceRetentionPolicy()
    enforceMaxCount()
  }

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
    persist()
    enforceCleanup()
    return seg.id
  }

  /** 标记某段为已读。 */
  function markRead(id: string): void {
    const seg = segments.value.find((s) => s.id === id)
    if (seg && !seg.read) {
      seg.read = true
      persist()
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
    if (changed) persist()
  }

  /** 删除一段对话。 */
  function removeSegment(id: string): void {
    segments.value = segments.value.filter((s) => s.id !== id)
    if (selectedSegmentId.value === id) {
      selectedSegmentId.value = segments.value[0]?.id ?? null
    }
    persist()
  }

  /** 清空所有段。 */
  function clearSegments(): void {
    segments.value = []
    selectedSegmentId.value = null
    persist()
  }

  /** 选定用于推送的段。 */
  function selectSegment(id: string): void {
    selectedSegmentId.value = id
  }

  return {
    segments,
    selectedSegmentId,
    loadSegments,
    enforceCleanup,
    addSegment,
    markRead,
    markAllRead,
    removeSegment,
    clearSegments,
    selectSegment,
  }
}
