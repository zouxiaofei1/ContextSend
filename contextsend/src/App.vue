<script setup lang="ts">
import { onMounted, ref, computed } from 'vue'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { useAppStore } from './stores/app'
import { useSettingsStore } from './stores/settings'
import { useLayout } from './composables/useLayout'
import { useContextCapture } from './composables/useContextCapture'
import { TAB_RECEIVE, TAB_DEVICES, TAB_SETTINGS } from './constants'
import AppSidebar from './components/AppSidebar.vue'
import BottomTabBar from './components/BottomTabBar.vue'
import TitleBar from './components/TitleBar.vue'
import ToastHost from './components/ToastHost.vue'
import ReceivePanel from './components/ReceivePanel.vue'
import DevicePanel from './components/DevicePanel.vue'
import SettingsPanel from './components/SettingsPanel.vue'

const app = useAppStore()
const { isPortrait, isCompact } = useLayout()

// 用户手动收起侧边栏（双击切换）；与窗口过窄触发的自动紧凑叠加生效
const manualCollapsed = ref(false)
const sidebarCompact = computed(() => isCompact.value || manualCollapsed.value)

// 全局：在窗口任意位置粘贴 / 拖入文本即匹配回本地会话并入库。
useContextCapture()

const activeTab = ref<string>(TAB_DEVICES)

/** 面板懒加载映射 */
const panelMap: Record<string, unknown> = {
  [TAB_RECEIVE]: ReceivePanel,
  [TAB_DEVICES]: DevicePanel,
  [TAB_SETTINGS]: SettingsPanel,
}

onMounted(() => {
  void app.init()

  const settings = useSettingsStore()
  if (settings.alwaysOnTop) {
    getCurrentWindow()
      .setAlwaysOnTop(true)
      .catch(() => {})
  }

  // 若用户启用了"启动时最小化"，则保持窗口隐藏（仅托盘图标可见）
  if (!settings.startMinimized) {
    void getCurrentWindow().show()
  }
})

function onSelectTab(tab: string): void {
  activeTab.value = tab
}

function onToggleSidebar(): void {
  manualCollapsed.value = !manualCollapsed.value
}
</script>

<template>
  <div class="app-root">
    <TitleBar :portrait="isPortrait" />
    <ToastHost />

    <!-- 横屏布局：左侧边栏 + 右侧内容 -->
    <div v-if="!isPortrait" class="app-layout">
      <AppSidebar
        :active-tab="activeTab"
        :compact="sidebarCompact"
        @select="onSelectTab"
        @toggle="onToggleSidebar"
      />
      <main class="app-main">
        <component :is="panelMap[activeTab]" />
      </main>
    </div>

    <!-- 竖屏布局：内容在上 + 底部 Tab 栏 -->
    <div v-else class="app-layout app-layout--portrait">
      <main class="app-main">
        <component :is="panelMap[activeTab]" />
      </main>
      <BottomTabBar :active-tab="activeTab" @select="onSelectTab" />
    </div>
  </div>
</template>

<style scoped>
.app-root {
  flex: 1;
  display: flex;
  height: 100vh;
  min-width: 0;
}

.app-layout {
  flex: 1;
  min-height: 0;
  display: flex;
  width: 100%;
}

.app-layout--portrait {
  flex-direction: column;
}

.app-main {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  background: var(--bg-primary);
  /* 顶部留出悬浮控制键/拖动区的安全区，避免内容被遮挡或顶部点击被拖动区拦截 */
  padding-top: 36px;
}
</style>
