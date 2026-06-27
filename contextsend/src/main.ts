import { createApp } from 'vue'
import { watch } from 'vue'
import { createPinia } from 'pinia'
import App from './App.vue'
import { createI18nInstance, registerI18n } from './i18n'
import { useSettingsStore } from './stores/settings'
import './styles.css'

const pinia = createPinia()
const app = createApp(App)

app.use(pinia)

// Pinia 安装后立即创建 settingsStore 以读取持久化设置
const settingsStore = useSettingsStore()

// 根据持久化设置初始化 i18n
const i18n = createI18nInstance(settingsStore.locale)
registerI18n(i18n)
app.use(i18n)

// 实际生效语言变化时（切换偏好或系统语言变更）同步到 i18n
watch(
  () => settingsStore.locale,
  (loc) => {
    i18n.global.locale.value = loc
  },
)

// 挂载前恢复主题
settingsStore.applyTheme()

app.mount('#app')

// 挂载后同步后端设置（autostart / minimizeToTray）
settingsStore.syncBackend()
