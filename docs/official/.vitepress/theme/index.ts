import { h, nextTick, watch } from 'vue'
import type { Theme } from 'vitepress'
import { useData } from 'vitepress'
import DefaultTheme from 'vitepress/theme'
import { createMermaidRenderer } from 'vitepress-mermaid-renderer'
import './style.css'

export default {
  extends: DefaultTheme,
  Layout: () => {
    const { isDark } = useData()

    const renderMermaid = () => {
      createMermaidRenderer({
        theme: isDark.value ? 'dark' : 'default',
        startOnLoad: false,
        flowchart: {
          useMaxWidth: true,
          htmlLabels: true
        }
      })
    }

    nextTick(renderMermaid)
    watch(() => isDark.value, renderMermaid)

    return h(DefaultTheme.Layout)
  }
} satisfies Theme
