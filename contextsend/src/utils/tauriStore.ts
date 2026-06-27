import { load, type Store } from '@tauri-apps/plugin-store'
import { invoke } from '@tauri-apps/api/core'
import { IPC } from '../constants'

let dataDirCache: string | null = null

async function getDataDir(): Promise<string> {
  if (!dataDirCache) {
    dataDirCache = await invoke<string>(IPC.GET_DATA_DIR)
  }
  return dataDirCache
}
export function createPersistentStore(file: string) {
  let store: Store | null = null

  async function ensure(): Promise<Store> {
    if (!store) {
      const dir = await getDataDir()
      const sep = dir.includes('\\') ? '\\' : '/'
      const absPath = `${dir}${sep}${file}`
      store = await load(absPath, { defaults: {}, autoSave: false })
    }
    return store
  }

  return {
    /** 读取某键；失败或不存在返回 undefined。 */
    async get<T>(key: string): Promise<T | undefined> {
      try {
        return await (await ensure()).get<T>(key)
      } catch (e) {
        console.error(`读取持久化失败 ${file}/${key}:`, e)
        return undefined
      }
    },
    /** 写入某键并落盘（失败仅记录）。 */
    async set(key: string, value: unknown): Promise<void> {
      try {
        const s = await ensure()
        await s.set(key, value)
        await s.save()
      } catch (e) {
        console.error(`持久化失败 ${file}/${key}:`, e)
      }
    },
  }
}
