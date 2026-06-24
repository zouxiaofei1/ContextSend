import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'

/** 与 Rust 端 `commands::AppInfo` 对应的应用信息。 */
export interface AppInfo {
  version: string
  platform: string
  adapters: string[]
}

/**
 * 应用级 Pinia store。
 *
 * Phase 0 仅演示「Pinia → invoke → Rust command」通信骨架；
 * 后续设备列表、配对状态、上下文等状态在此扩展（或拆分为独立 store）。
 */
export const useAppStore = defineStore('app', () => {
  const info = ref<AppInfo | null>(null)
  const error = ref<string | null>(null)
  const loading = ref(false)

  async function loadAppInfo(): Promise<void> {
    loading.value = true
    error.value = null
    try {
      info.value = await invoke<AppInfo>('get_app_info')
    } catch (e) {
      error.value = String(e)
    } finally {
      loading.value = false
    }
  }

  return { info, error, loading, loadAppInfo }
})
