import { createApp } from 'vue'
import { createPinia } from 'pinia'
import App from './App.vue'
import { createI18nInstance } from './i18n'
import { useSettingsStore } from './stores/settings'
import './styles.css'

const pinia = createPinia()
const app = createApp(App)

app.use(pinia)

// Pinia 安装后立即创建 settingsStore 以读取持久化设置
const settingsStore = useSettingsStore()

// 根据持久化设置初始化 i18n
const i18n = createI18nInstance(settingsStore.locale)
app.use(i18n)

// 挂载前恢复主题
settingsStore.applyTheme()

app.mount('#app')

// 挂载后同步后端设置（autostart / minimizeToTray）
settingsStore.syncBackend()
