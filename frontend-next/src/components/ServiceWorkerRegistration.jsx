'use client'

import { useEffect } from 'react'

export default function ServiceWorkerRegistration() {
  useEffect(() => {
    if (!('serviceWorker' in navigator)) return
    let mounted = true
    navigator.serviceWorker.register('/prefetch-sw.js').catch(() => {
      if (mounted) {
        console.warn('Service worker registration failed')
      }
    })

    return () => {
      mounted = false
    }
  }, [])

  return null
}
