import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { IPC, STORE_FILE, STORE_KEY } from '../../constants'
import { createPersistentStore } from '../../utils/tauriStore'
import type { Device } from '../types'

/** 持久化的设备记录（不含运行时的 `online` 状态——恢复时一律先视为离线）。 */
type StoredDevice = Pick<Device, 'id' | 'name' | 'os' | 'ip' | 'lastSync'>

/**
 * 设备发现模块：维护设备列表快照。
 *
 * 与早期实现的区别：
 * - **离线不删除**：`deviceLost` 改为标记离线（沉底显示），并跨重启记住已知设备；
 * - **上次同步时间**：成功推送/接收后记录时间戳，按设备 uuid 持久化到 `devices.json`。
 */
export function useDevices() {
  const devices = ref<Device[]>([])
  const store = createPersistentStore(STORE_FILE.DEVICES)

  /** 把当前设备名册（去掉 online）落盘。 */
  function persist(): void {
    const roster: StoredDevice[] = devices.value.map((d) => ({
      id: d.id,
      name: d.name,
      os: d.os,
      ip: d.ip,
      lastSync: d.lastSync,
    }))
    void store.set(STORE_KEY.DEVICES, roster)
  }

  /** 从磁盘恢复已知设备：一律先标记离线，待发现事件/快照再置为在线。 */
  async function loadDevices(): Promise<void> {
    const saved = await store.get<StoredDevice[]>(STORE_KEY.DEVICES)
    if (Array.isArray(saved)) {
      devices.value = saved.map((d) => ({ ...d, online: false }))
    }
  }

  /** 用后端权威快照（仅含在线设备）合并：已知设备保留，逐台标记在线/离线。 */
  async function refreshDevices(): Promise<void> {
    try {
      const live = await invoke<Device[]>(IPC.LIST_DEVICES)
      const liveIds = new Set(live.map((d) => d.id))
      for (const d of devices.value) {
        if (!liveIds.has(d.id)) d.online = false
      }
      for (const d of live) upsertDevice({ ...d, online: true })
    } catch {
      /* 网络尚未就绪时忽略 */
    }
  }

  /** 插入或更新一台设备（`deviceFound` / 快照）。合并时保留已持久化的 `lastSync`。 */
  function upsertDevice(dev: Device): void {
    const idx = devices.value.findIndex((d) => d.id === dev.id)
    if (idx >= 0) {
      const prev = devices.value[idx]
      devices.value[idx] = { ...prev, ...dev, lastSync: dev.lastSync ?? prev.lastSync }
    } else {
      devices.value.push(dev)
    }
    persist()
  }

  /** 标记某设备离线（对应 `deviceLost`）：不再从列表移除，沉底显示。 */
  function markOffline(id: string): void {
    const d = devices.value.find((x) => x.id === id)
    if (d) d.online = false
  }

  /** 记录与某设备的最近一次成功同步时间并持久化。设备不在册时忽略。 */
  function recordSync(id: string, ts: number): void {
    const d = devices.value.find((x) => x.id === id)
    if (d) {
      d.lastSync = ts
      persist()
    }
  }

  /**
   * 忘记某设备：从名册移除并落盘（清掉持久化的 lastSync 等记录）。
   * 仅对离线设备有意义——在线设备会被发现事件/快照立即重新加入。
   */
  function forgetDevice(id: string): void {
    devices.value = devices.value.filter((d) => d.id !== id)
    persist()
  }

  return {
    devices,
    loadDevices,
    refreshDevices,
    upsertDevice,
    markOffline,
    recordSync,
    forgetDevice,
  }
}
