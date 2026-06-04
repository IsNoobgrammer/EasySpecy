import { defineConfig } from 'vitepress'

export default defineConfig({
  title: 'EasySpecy',
  description: 'Free, open-source screen recorder with cinematic auto-zoom and cursor effects',
  base: '/EasySpecy/',
  appearance: 'dark',
  
  head: [
    ['link', { rel: 'icon', href: '/EasySpecy/logo.png' }],
    ['link', { rel: 'preconnect', href: 'https://fonts.googleapis.com' }],
    ['link', { rel: 'preconnect', href: 'https://fonts.gstatic.com', crossorigin: '' }],
    ['link', { rel: 'stylesheet', href: 'https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700;800&family=JetBrains+Mono:wght@400;500;600&display=swap' }],
    ['meta', { name: 'theme-color', content: '#00e88a' }],
    ['meta', { property: 'og:type', content: 'website' }],
    ['meta', { property: 'og:locale', content: 'en' }],
    ['meta', { property: 'og:title', content: 'EasySpecy — Record like a pro. Pay like it\'s 2005.' }],
    ['meta', { property: 'og:description', content: 'Free, cross-platform screen recorder with cinematic auto-zoom, cursor effects, and webcam overlay. Built with Rust + Tauri.' }],
    ['meta', { property: 'og:image', content: 'https://easyspecy.github.io/EasySpecy/og-image.png' }],
  ],
  
  themeConfig: {
    logo: '/logo.png',
    
    nav: [
      { text: 'Guide', link: '/guide/introduction' },
      { text: 'Config', link: '/config/overview' },
      { text: 'GitHub', link: 'https://github.com/IsNoobgrammer/EasySpecy' }
    ],
    
    sidebar: {
      '/guide/': [
        {
          text: 'Getting Started',
          items: [
            { text: 'Introduction', link: '/guide/introduction' },
            { text: 'Installation', link: '/guide/installation' },
            { text: 'Quick Start', link: '/guide/quick-start' }
          ]
        },
        {
          text: 'Core Features',
          items: [
            { text: 'Screen Capture', link: '/guide/screen-capture' },
            { text: 'Cursor Trails', link: '/guide/cursor-settings' },
            { text: 'Keyboard Overlay', link: '/guide/keyboard-settings' },
            { text: 'Webcam Overlay', link: '/guide/webcam-settings' },
            { text: 'Auto-Zoom', link: '/guide/auto-zoom' },
            { text: 'Hotkeys', link: '/guide/hotkeys' }
          ]
        }
      ],
      '/config/': [
        {
          text: 'Configuration',
          items: [
            { text: 'Overview', link: '/config/overview' }
          ]
        }
      ]
    },
    
    socialLinks: [
      { icon: 'github', link: 'https://github.com/IsNoobgrammer/EasySpecy' }
    ],
    
    editLink: {
      pattern: 'https://github.com/IsNoobgrammer/EasySpecy/edit/master/docs/:path',
      text: 'Edit this page on GitHub'
    },
    
    footer: {
      message: 'Released under the MIT License.',
      copyright: 'Copyright © 2026-present Shaurya'
    },
    
    search: {
      provider: 'local'
    },
    
    outline: {
      level: [2, 3],
      label: 'On this page'
    }
  },
  
  markdown: {
    lineNumbers: true,
    theme: {
      light: 'github-light',
      dark: 'github-dark'
    }
  }
})
