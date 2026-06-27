<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useSettingsStore } from '../stores/settings'
import { useAppStore } from '../stores/app'
import GeneralSettings from './settings/GeneralSettings.vue'
import AdvancedSettings from './settings/AdvancedSettings.vue'
import AboutSection from './settings/AboutSection.vue'
import LanguageSettings from './settings/LanguageSettings.vue'
import AdapterSettings from './settings/AdapterSettings.vue'
import AdapterDetailSettings from './settings/AdapterDetailSettings.vue'
import type { AdapterInfo } from '../stores/app'

const settings = useSettingsStore()
const app = useAppStore()

type SubView = 'root' | 'language' | 'adapter'
const subView = ref<SubView>('root')
// 前进（进入子页）从右滑入，后退（返回）从左滑入；切换动画时长 150ms。
const transitionName = ref<'slide-forward' | 'slide-back'>('slide-forward')

const adapters = ref<AdapterInfo[]>([])
const selectedAdapter = ref<AdapterInfo | null>(null)

async function reloadAdapters(): Promise<void> {
  try {
    adapters.value = await app.listAdapters()
    // 子页打开时同步刷新当前选中适配器的信息（保存后回填）。
    if (selectedAdapter.value) {
      selectedAdapter.value =
        adapters.value.find((a) => a.name === selectedAdapter.value?.name) ?? selectedAdapter.value
    }
  } catch {
    // 后端未就绪时忽略，下次进入设置再试。
  }
}

onMounted(reloadAdapters)

function openLanguage(): void {
  transitionName.value = 'slide-forward'
  subView.value = 'language'
}

function openAdapter(name: string): void {
  const found = adapters.value.find((a) => a.name === name)
  if (!found) return
  selectedAdapter.value = found
  transitionName.value = 'slide-forward'
  subView.value = 'adapter'
}

function back(): void {
  transitionName.value = 'slide-back'
  subView.value = 'root'
}
</script>

<template>
  <div class="settings-root">
    <Transition :name="transitionName">
      <div v-if="subView === 'root'" key="root" class="subpage">
        <AdapterSettings :adapters="adapters" @open="openAdapter" />
        <GeneralSettings @open-language="openLanguage" />
        <AdvancedSettings v-if="settings.showAdvanced" />
        <AboutSection />
      </div>
      <LanguageSettings
        v-else-if="subView === 'language'"
        key="language"
        class="subpage"
        @back="back"
      />
      <AdapterDetailSettings
        v-else
        key="adapter"
        class="subpage"
        :adapter="selectedAdapter!"
        @back="back"
        @saved="reloadAdapters"
      />
    </Transition>
  </div>
</template>

<style scoped>
.settings-root {
  position: relative;
  flex: 1;
  min-height: 0;
  overflow: hidden;
  user-select: none;
}

.subpage {
  position: absolute;
  inset: 0;
  padding: 1.5rem 2rem;
  overflow-y: auto;
}

/* 150ms 横向滑动 + 淡入淡出 */
.slide-forward-enter-active,
.slide-forward-leave-active,
.slide-back-enter-active,
.slide-back-leave-active {
  transition:
    transform 0.15s ease,
    opacity 0.15s ease;
}

.slide-forward-enter-from {
  transform: translateX(100%);
  opacity: 0;
}
.slide-forward-leave-to {
  transform: translateX(-30%);
  opacity: 0;
}

.slide-back-enter-from {
  transform: translateX(-30%);
  opacity: 0;
}
.slide-back-leave-to {
  transform: translateX(100%);
  opacity: 0;
}
</style>
