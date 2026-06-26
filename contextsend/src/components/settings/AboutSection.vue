<script setup lang="ts">
import { useAppStore } from '../../stores/app'
import { GITHUB_URL } from '../../constants'
import appIcon from '../../assets/app-icon.png'

const app = useAppStore()

function openGitHub(): void {
  window.open(GITHUB_URL, '_blank')
}
</script>

<template>
  <section v-if="app.info" class="about-section">
    <img :src="appIcon" alt="ContextSend" class="about-icon" />
    <h1 class="about-title">ContextSend</h1>
    <div class="about-meta">
      <a class="about-link" @click.prevent="openGitHub">GitHub</a>
      <span
        class="about-version"
        :title="`Build ${app.info.buildNumber}`"
      >Ver {{ app.info.version }} ({{ app.info.platform }})</span>
      <span class="about-license">MIT</span>
    </div>
  </section>
</template>

<style scoped>
.about-section {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.8rem;
  padding: 2rem 0 1rem;
  margin-top: 1.5rem;
  border-top: 1px solid var(--border);
}

.about-icon {
  width: 96px;
  height: 96px;
  border-radius: 22px;
  user-select: none;
  -webkit-user-drag: none;
}

.about-title {
  margin: 0;
  font-size: 1.6rem;
  font-weight: 700;
  color: var(--text-primary);
  letter-spacing: -0.01em;
}

.about-meta {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
  max-width: 360px;
  margin-top: 0.4rem;
  font-size: 0.8rem;
  color: var(--text-secondary);
}

.about-link {
  color: var(--accent);
  text-decoration: none;
  cursor: pointer;
}

.about-link:hover {
  text-decoration: underline;
}

.about-version {
  flex: 1;
  text-align: center;
}

.about-license {
  font-weight: 500;
}
</style>
