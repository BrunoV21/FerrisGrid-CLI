import { h, nextTick, watch } from 'vue'
import type { Theme } from 'vitepress'
import { useData } from 'vitepress'
import DefaultTheme from 'vitepress/theme'
import { createMermaidRenderer } from 'vitepress-mermaid-renderer'
import './style.css'

const rawContentBase = 'https://raw.githubusercontent.com/BrunoV21/FerrisGrid-CLI/main/docs/official/'

export default {
  extends: DefaultTheme,
  Layout: () => {
    const { isDark, page } = useData()

    const RawMarkdownLink = () => {
      if (page.value.isNotFound || !page.value.relativePath) {
        return null
      }

      const rawUrl = `${rawContentBase}${page.value.relativePath}`

      return h('div', { class: 'raw-markdown-link' }, [
        h(
          'a',
          {
            href: rawUrl,
            target: '_blank',
            rel: 'noopener',
            'data-raw-markdown-link': page.value.relativePath
          },
          'View raw Markdown'
        )
      ])
    }

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

    return h(DefaultTheme.Layout, null, {
      'doc-before': RawMarkdownLink,
      'home-features-after': RawMarkdownLink
    })
  }
} satisfies Theme
