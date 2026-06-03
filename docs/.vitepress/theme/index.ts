import DefaultTheme from 'vitepress/theme'
import type { Theme } from 'vitepress'
import LandingPage from './LandingPage.vue'
import './custom.css'

export default {
  extends: DefaultTheme,
  enhanceApp({ app, router }) {
    app.component('LandingPage', LandingPage)

    // Smooth scroll behavior
    if (typeof document !== 'undefined') {
      document.documentElement.style.scrollBehavior = 'smooth'
    }

    // Page transition on route change
    router.onAfterRouteChanged = (to) => {
      if (typeof document !== 'undefined') {
        const doc = document.querySelector('.VPDoc') as HTMLElement
        if (doc) {
          doc.classList.remove('page-enter')
          void doc.offsetWidth // Force reflow
          doc.classList.add('page-enter')
        }
      }
    }
  }
} as Theme

