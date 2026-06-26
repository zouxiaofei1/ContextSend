import { load, type Store } from '@tauri-apps/plugin-store'

/**
 * 懒加载的 Tauri plugin-store（磁盘 JSON）包装：首次访问时 `load`，之后复用同一句柄。
 * get/set 失败仅 `console.error` 记录、不抛出，避免阻断 UI。
 *
 * 抽出自原 `stores/app.ts` 中 segments / permissions 各自重复的
 * persist / load 逻辑（四份模板合一）。
 */
export function createPersistentStore(file: string) {
  let store: Store | null = null

  async function ensure(): Promise<Store> {
    if (!store) store = await load(file, { defaults: {}, autoSave: false })
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
