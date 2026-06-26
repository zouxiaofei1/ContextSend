<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useSettingsStore } from '../../stores/settings'
import { PORT_MIN, PORT_MAX, TIMEOUT_MIN, TIMEOUT_MAX, RETENTION_OPTIONS } from '../../constants'
import type { RetentionValue } from '../../constants'
import SettingsSection from './SettingsSection.vue'
import SettingRow from './SettingRow.vue'
import SettingToggle from './SettingToggle.vue'
import SettingSelect from './SettingSelect.vue'
import SettingNumber from './SettingNumber.vue'

const settings = useSettingsStore()
const { t } = useI18n()

const retentionOptions = computed(() =>
  RETENTION_OPTIONS.map((opt) => ({ value: opt.value, label: t(opt.labelKey) })),
)
</script>

<template>
  <SettingsSection :title="t('settings.advanced.title')">
    <!-- 自定义网络端口 -->
    <SettingRow :label="t('settings.advanced.port')">
      <SettingNumber
        :model-value="settings.customPort"
        :empty-value="0"
        :placeholder="t('settings.advanced.portPlaceholder')"
        :min="PORT_MIN"
        :max="PORT_MAX"
        @update:model-value="settings.setCustomPort($event)"
      />
    </SettingRow>

    <!-- 连接超时 -->
    <SettingRow :label="t('settings.advanced.timeout')">
      <SettingNumber
        :model-value="settings.connectionTimeout"
        :placeholder="t('settings.advanced.timeoutPlaceholder')"
        :min="TIMEOUT_MIN"
        :max="TIMEOUT_MAX"
        :unit="t('settings.advanced.seconds')"
        @update:model-value="settings.setConnectionTimeout($event)"
      />
    </SettingRow>

    <!-- 窗口置顶 -->
    <SettingRow :label="t('settings.advanced.alwaysOnTop')">
      <SettingToggle
        :model-value="settings.alwaysOnTop"
        @update:model-value="settings.toggleAlwaysOnTop()"
      />
    </SettingRow>

    <!-- 启动时最小化到托盘 -->
    <SettingRow :label="t('settings.advanced.startMinimized')">
      <SettingToggle
        :model-value="settings.startMinimized"
        @update:model-value="settings.toggleStartMinimized()"
      />
    </SettingRow>

    <!-- 对话保存期限 -->
    <SettingRow :label="t('settings.advanced.retention')">
      <SettingSelect
        :model-value="settings.conversationRetention"
        :options="retentionOptions"
        @update:model-value="settings.setConversationRetention($event as RetentionValue)"
      />
    </SettingRow>

    <!-- 最大缓存对话条数 -->
    <SettingRow :label="t('settings.advanced.maxConversationCount')">
      <SettingNumber
        :model-value="settings.maxConversationCount"
        :empty-value="-1"
        :placeholder="t('settings.advanced.maxConversationCountPlaceholder')"
        :min="-1"
        :max="9999"
        @update:model-value="settings.setMaxConversationCount($event)"
      />
    </SettingRow>
  </SettingsSection>
</template>
