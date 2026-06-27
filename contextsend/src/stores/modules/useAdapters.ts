import { invoke } from '@tauri-apps/api/core'
import { IPC, ADAPTER_CHATBOX } from '../../constants'
import { translate as t } from '../../i18n'
import { useToastStore } from '../toast'
import type { Conversation, AdapterInfo, AdapterConfig } from '../types'

/**
 * 适配器导入 / 导出模块：OpenAI Compatible JSON 互转，上下文片段匹配，
 * 以及把对话写入本机 Chat AI 应用（Jan / ChatBox）。均为无状态的 IPC 封装。
 */
export function useAdapters() {
  const toast = useToastStore()

  /** 导入 OpenAI Compatible JSON 文本。 */
  async function importOpenai(json: string): Promise<Conversation> {
    return await invoke<Conversation>(IPC.IMPORT_OPENAI, { json })
  }

  /** 导出对话为 OpenAI Compatible JSON 文本。 */
  async function exportOpenai(conversation: Conversation): Promise<string> {
    return await invoke<string>(IPC.EXPORT_OPENAI, { conversation })
  }

  /**
   * 把一段复制 / 拖入的上下文片段匹配回本地应用里的完整会话（导出方向）。
   * 命中则返回整条会话，未命中则把片段包成占位会话；片段过短后端会报错。
   */
  async function matchContext(
    snippet: string,
  ): Promise<{ matched: boolean; app: string | null; score: number; conversation: Conversation }> {
    return await invoke(IPC.MATCH_CONTEXT, { snippet })
  }
  async function importToApp(conversation: Conversation, appName: string): Promise<boolean> {
    try {
      await invoke<{ app: string; threadId: string }>(IPC.IMPORT_TO_APP, {
        app: appName,
        conversation,
      })
      const key =
        appName.toLowerCase() === ADAPTER_CHATBOX.toLowerCase()
          ? 'toast.importAppSuccessChatBox'
          : 'toast.importAppSuccess'
      toast.success(t(key, { app: appName }))
      return true
    } catch (e) {
      toast.error(t('toast.importAppFailed', { app: appName, error: String(e) }))
      return false
    }
  }

  /** 列出所有内置适配器的探测状态与当前配置（供「设置 → 适配器」页）。 */
  async function listAdapters(): Promise<AdapterInfo[]> {
    return await invoke<AdapterInfo[]>(IPC.LIST_ADAPTERS)
  }

  /** 写入某适配器的配置覆盖（数据目录 / 安装目录 / 端口）。 */
  async function setAdapterConfig(name: string, config: AdapterConfig): Promise<void> {
    await invoke(IPC.SET_ADAPTER_CONFIG, { name, config })
  }

  return { importOpenai, exportOpenai, matchContext, importToApp, listAdapters, setAdapterConfig }
}
