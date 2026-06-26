import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { IPC } from '../../constants'
import type { Device } from '../types'

/**
 * 设备发现模块：维护设备列表快照，并提供 `net-event` 事件落库所需的增删方法。
 */
export function useDevices() {
  const devices = ref<Device[]>([])

  /** 重新拉取设备列表快照（权威覆盖）。网络尚未就绪时忽略。 */
  async function refreshDevices(): Promise<void> {
    try {
      devices.value = await invoke<Device[]>(IPC.LIST_DEVICES)
    } catch {
      /* 网络尚未就绪时忽略 */
    }
  }

  /** 插入或更新一台设备（对应 `deviceFound` 事件）。 */
  function upsertDevice(dev: Device): void {
    const idx = devices.value.findIndex((d) => d.id === dev.id)
    if (idx >= 0) devices.value[idx] = dev
    else devices.value.push(dev)
  }

  /** 移除一台设备（对应 `deviceLost` 事件）。 */
  function removeDevice(id: string): void {
    devices.value = devices.value.filter((d) => d.id !== id)
  }

  return { devices, refreshDevices, upsertDevice, removeDevice }
}
