/**
 * Tauri IPC invoke 命令名。
 * 与 Rust `generate_handler![]` 注册列表一一对应。
 */
export const IPC = {
  GET_APP_INFO: 'get_app_info',
  GET_DATA_DIR: 'get_data_dir',
  GET_SELF_IDENTITY: 'get_self_identity',
  LIST_DEVICES: 'list_devices',
  RENAME_SELF: 'rename_self',
  CONNECT_PAIR: 'connect_pair',
  PUSH_CONVERSATION: 'push_conversation',
  ACCEPT_INCOMING: 'accept_incoming',
  REJECT_PAIRING: 'reject_pairing',
  IMPORT_OPENAI: 'import_openai',
  EXPORT_OPENAI: 'export_openai',
  IMPORT_TO_APP: 'import_to_app',
  MATCH_CONTEXT: 'match_context',
  SAVE_EXPORT: 'save_export',
  SET_MINIMIZE_TO_TRAY: 'set_minimize_to_tray',
  SET_NETWORK_PORT: 'set_network_port',
  SET_CONNECTION_TIMEOUT: 'set_connection_timeout',
  SET_GLOBAL_SHORTCUT: 'set_global_shortcut',
} as const

/** Tauri 事件名（后端 emit → 前端 listen） */
export const EVENT = {
  NET_READY: 'net-ready',
  NET_EVENT: 'net-event',
  NET_ERROR: 'net-error',
} as const

/** Tauri plugin-store 磁盘文件名 */
export const STORE_FILE = {
  SEGMENTS: 'segments.json',
  PERMISSIONS: 'permissions.json',
  DEVICES: 'devices.json',
} as const

/** Tauri plugin-store 数据键 */
export const STORE_KEY = {
  SEGMENTS: 'segments',
  PERMISSIONS: 'permissions',
  DEVICES: 'devices',
} as const

/** 浏览器 localStorage 键 */
export const LS_SETTINGS = 'contextsend_settings'
