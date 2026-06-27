import { ref } from 'vue'
import { STORE_FILE, STORE_KEY } from '../../constants'
import { createPersistentStore } from '../../utils/tauriStore'
import type { PermissionLevel } from '../types'
export function usePermissions() {
  const permissions = ref<Record<string, PermissionLevel>>({})
  const store = createPersistentStore(STORE_FILE.PERMISSIONS)

  /** 从磁盘恢复权限表。 */
  async function loadPermissions(): Promise<void> {
    const saved = await store.get<Record<string, PermissionLevel>>(STORE_KEY.PERMISSIONS)
    if (saved && typeof saved === 'object') permissions.value = saved
  }

  /** 读取某设备的权限等级（未记录则为默认 Level 0 陌生人）。 */
  function permissionOf(id: string): PermissionLevel {
    return permissions.value[id] ?? 0
  }

  /** 设置某设备的权限等级并落盘。Level 2 的升级须先经配对码验证。 */
  function setPermission(id: string, level: PermissionLevel): void {
    permissions.value[id] = level
    void store.set(STORE_KEY.PERMISSIONS, permissions.value)
  }

  return { permissions, loadPermissions, permissionOf, setPermission }
}
