import { defineConfig } from 'vitepress'

export default defineConfig({
  title: 'FerrisGrid',
  description: 'Single-step visual computer control for local agents.',
  appearance: 'force-dark',

  head: [
    ['link', { rel: 'preconnect', href: 'https://fonts.googleapis.com' }],
    ['link', { rel: 'preconnect', href: 'https://fonts.gstatic.com', crossorigin: '' }],
    ['link', { href: 'https://fonts.googleapis.com/css2?family=JetBrains+Mono:wght@400;500;600;700&display=swap', rel: 'stylesheet' }]
  ],

  themeConfig: {
    nav: [
      { text: 'Home', link: '/' },
      { text: 'Get Started', link: '/getting-started/' },
      { text: 'Commands', link: '/commands/' },
      { text: 'Workspaces', link: '/workspaces/docker' },
      { text: 'Concepts', link: '/concepts/architecture' },
      { text: 'Roadmap', link: '/roadmap' }
    ],

    sidebar: [
      {
        text: '// GETTING STARTED',
        items: [
          { text: 'Overview', link: '/getting-started/' },
          { text: 'Installation', link: '/getting-started/installation' },
          { text: 'First Run', link: '/getting-started/first-run' }
        ]
      },
      {
        text: '// COMMANDS',
        items: [
          { text: 'Overview', link: '/commands/' },
          { text: 'observe', link: '/commands/observe' },
          { text: 'act', link: '/commands/act' },
          { text: 'doctor', link: '/commands/doctor' },
          { text: 'recap', link: '/commands/recap' },
          { text: 'clear', link: '/commands/clear' }
        ]
      },
      {
        text: '// WORKSPACES',
        items: [
          { text: 'Docker Linux Workspace', link: '/workspaces/docker' }
        ]
      },
      {
        text: '// CONCEPTS',
        items: [
          { text: 'Architecture', link: '/concepts/architecture' },
          { text: 'Agent Protocol', link: '/concepts/agent-protocol' },
          { text: 'Local Traces', link: '/concepts/local-traces' }
        ]
      },
      {
        text: '// MORE',
        items: [
          { text: 'Roadmap', link: '/roadmap' }
        ]
      }
    ],

    socialLinks: [
      { icon: 'github', link: 'https://github.com/BrunoV21/FerrisPilot' }
    ],

    footer: {
      message: 'FerrisGrid - terminal-first visual control for local agents'
    },

    search: {
      provider: 'local'
    }
  }
})
