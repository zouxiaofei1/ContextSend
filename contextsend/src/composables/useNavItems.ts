import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { TAB_RECEIVE, TAB_DEVICES, TAB_SETTINGS } from '../constants'
import iconReceive from '../assets/icon-receive.svg?raw'
import iconDevices from '../assets/icon-devices.svg?raw'
import iconSettings from '../assets/icon-settings.svg?raw'

export function useNavItems() {
  const { t } = useI18n()

  const navItems = computed(() => [
    { id: TAB_DEVICES, icon: iconDevices, label: t('sidebar.devices') },
    { id: TAB_RECEIVE, icon: iconReceive, label: t('sidebar.receive') },
    { id: TAB_SETTINGS, icon: iconSettings, label: t('sidebar.settings') },
  ])

  return { navItems }
}
