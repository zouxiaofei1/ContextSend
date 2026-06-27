<script setup lang="ts">
import { ref, watch, onMounted, onUnmounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { open } from '@tauri-apps/plugin-dialog'
import { useAppStore } from '../../stores/app'
import { useToastStore } from '../../stores/toast'
import { ADAPTER_LOGOS, PORT_MIN, PORT_MAX } from '../../constants'
import type { AdapterInfo, AdapterConfig } from '../../stores/app'
import SettingsSection from './SettingsSection.vue'
import SettingRow from './SettingRow.vue'

const props = defineProps<{ adapter: AdapterInfo }>()
const emit = defineEmits<{ back: []; saved: [] }>()

const app = useAppStore()
const toast = useToastStore()
const { t } = useI18n()

// 各字段的本地可编辑副本（端口用字符串便于「留空＝沿用默认」）。
const dataDir = ref('')
const installDir = ref('')
const port = ref('')

// 适配器切换 / 父级刷新时回填本地状态。
watch(
  () => props.adapter,
  (a) => {
    dataDir.value = a.dataDir ?? ''
    installDir.value = a.installDir ?? ''
    port.value = a.port != null ? String(a.port) : ''
  },
  { immediate: true },
)

function has(field: 'dataDir' | 'installDir' | 'port'): boolean {
  return props.adapter.fields.includes(field)
}

function logo(): string | undefined {
  return ADAPTER_LOGOS[props.adapter.name.toLowerCase()]
}

/** 弹出系统目录选择器，选中后写回对应字段并保存。 */
async function browse(target: 'dataDir' | 'installDir'): Promise<void> {
  const current = target === 'dataDir' ? dataDir.value : installDir.value
  const picked = await open({
    directory: true,
    multiple: false,
    defaultPath: current || undefined,
  })
  if (typeof picked === 'string') {
    if (target === 'dataDir') dataDir.value = picked
    else installDir.value = picked
    await save()
  }
}

/** 收集当前字段值，写入后端配置覆盖；端口非法时提示并不保存。 */
async function save(): Promise<void> {
  const config: AdapterConfig = {}
  if (has('dataDir')) config.dataDir = dataDir.value.trim() || null
  if (has('installDir')) config.installDir = installDir.value.trim() || null
  if (has('port')) {
    const raw = port.value.trim()
    if (raw === '') {
      config.port = null
    } else {
      const n = Number(raw)
      if (!Number.isInteger(n) || n < PORT_MIN || n > PORT_MAX) {
        toast.error(t('settings.adapters.portInvalid', { min: PORT_MIN, max: PORT_MAX }))
        return
      }
      config.port = n
    }
  }
  try {
    await app.setAdapterConfig(props.adapter.name, config)
    emit('saved')
  } catch (e) {
    toast.error(t('settings.adapters.saveFailed', { error: String(e) }))
  }
}

function onKeydown(e: KeyboardEvent): void {
  if (e.key === 'Escape') emit('back')
}
onMounted(() => document.addEventListener('keydown', onKeydown))
onUnmounted(() => document.removeEventListener('keydown', onKeydown))
</script>

<template>
  <div class="adapter-detail">
    <header class="adapter-detail__header">
      <button class="back-btn" :title="t('common.back')" @click="emit('back')">
        <svg viewBox="0 0 24 24" width="22" height="22" aria-hidden="true">
          <path
            d="M15 6l-6 6 6 6"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
          />
        </svg>
      </button>
      <img v-if="logo()" class="adapter-detail__logo" :src="logo()" :alt="adapter.name" />
      <h2 class="adapter-detail__title">{{ adapter.name }}</h2>
      <span class="adapter-detail__status" :class="adapter.installed ? 'is-online' : 'is-offline'">
        {{
          adapter.installed ? t('settings.adapters.installed') : t('settings.adapters.notInstalled')
        }}
      </span>
    </header>

    <SettingsSection :title="t('settings.adapters.configTitle')">
      <!-- 数据目录 -->
      <SettingRow v-if="has('dataDir')" :label="t('settings.adapters.dataDir')">
        <div class="path-control">
          <input
            v-model="dataDir"
            class="path-input"
            :placeholder="t('settings.adapters.autoDetected')"
            @blur="save"
            @keyup.enter="($event.target as HTMLInputElement).blur()"
          />
          <button class="ghost small" @click="browse('dataDir')">
            {{ t('settings.adapters.browse') }}
          </button>
        </div>
      </SettingRow>

      <!-- 程序安装目录 -->
      <SettingRow v-if="has('installDir')" :label="t('settings.adapters.installDir')">
        <div class="path-control">
          <input
            v-model="installDir"
            class="path-input"
            :placeholder="t('settings.adapters.notSet')"
            @blur="save"
            @keyup.enter="($event.target as HTMLInputElement).blur()"
          />
          <button class="ghost small" @click="browse('installDir')">
            {{ t('settings.adapters.browse') }}
          </button>
        </div>
      </SettingRow>

      <!-- 端口 -->
      <SettingRow v-if="has('port')" :label="t('settings.adapters.port')">
        <input
          v-model="port"
          type="number"
          class="port-input"
          :min="PORT_MIN"
          :max="PORT_MAX"
          :placeholder="t('settings.adapters.defaultPort')"
          @blur="save"
          @keyup.enter="($event.target as HTMLInputElement).blur()"
        />
      </SettingRow>
    </SettingsSection>
  </div>
</template>

<style scoped>
.adapter-detail {
  display: flex;
  flex-direction: column;
}

.adapter-detail__header {
  display: flex;
  align-items: center;
  gap: 0.6rem;
  margin-bottom: 1.25rem;
}

.back-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  padding: 0;
  border: none;
  border-radius: 8px;
  background: transparent;
  color: var(--text-primary);
  cursor: pointer;
  transition: background 0.12s ease;
}

.back-btn:hover {
  background: var(--bg-secondary);
}

.adapter-detail__logo {
  width: 28px;
  height: 28px;
  border-radius: 7px;
  object-fit: contain;
}

.adapter-detail__title {
  margin: 0;
  font-size: 1.25rem;
  font-weight: 600;
  color: var(--text-primary);
}

.adapter-detail__status {
  font-size: 0.72rem;
  padding: 0.12rem 0.5rem;
  border-radius: 999px;
}

.adapter-detail__status.is-online {
  color: var(--accent);
  background: color-mix(in srgb, var(--accent) 14%, transparent);
}

.adapter-detail__status.is-offline {
  color: var(--text-secondary);
  background: var(--bg-secondary);
}

.path-control {
  display: flex;
  gap: 0.5rem;
  align-items: center;
}

.path-input {
  width: 280px;
}

.port-input {
  width: 120px;
  text-align: right;
}
</style>
