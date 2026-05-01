'use client'

import { useRouter } from 'next/navigation'
import { useEffect, useRef } from 'react'

const MIN_SPEED = 0.4
const MAX_ETA_MS = 220
const MAX_DISTANCE = 340

function isInternalHref(rawHref) {
  if (!rawHref) return false
  if (rawHref.startsWith('#')) return false
  if (rawHref.startsWith('http')) return rawHref.startsWith(window.location.origin)
  return rawHref.startsWith('/')
}

function getLinkCandidates() {
  return Array.from(document.querySelectorAll('a[data-predictive="true"]'))
}

function toCenter(rect) {
  return {
    x: rect.left + rect.width / 2,
    y: rect.top + rect.height / 2
  }
}

export default function PredictivePrefetchProvider({ children }) {
  const router = useRouter()
  const prefetched = useRef(new Set())
  const history = useRef([])

  useEffect(() => {
    const prefetchUrl = (href) => {
      if (!href || prefetched.current.has(href) || !isInternalHref(href)) return
      prefetched.current.add(href)
      router.prefetch(href)
      if (navigator.serviceWorker?.controller) {
        navigator.serviceWorker.controller.postMessage({
          type: 'PREFETCH_URL',
          href
        })
      }
    }

    const onPointerMove = (event) => {
      const now = performance.now()
      const points = history.current
      points.push({ x: event.clientX, y: event.clientY, t: now })
      if (points.length > 5) points.shift()
      if (points.length < 2) return

      const last = points[points.length - 1]
      const prev = points[points.length - 2]
      const dt = Math.max(1, last.t - prev.t)
      const vx = (last.x - prev.x) / dt
      const vy = (last.y - prev.y) / dt
      const speed = Math.hypot(vx, vy)
      if (speed < MIN_SPEED) return

      const links = getLinkCandidates()
      for (const link of links) {
        const rect = link.getBoundingClientRect()
        const center = toCenter(rect)
        const dx = center.x - last.x
        const dy = center.y - last.y
        const distance = Math.hypot(dx, dy)
        if (distance > MAX_DISTANCE) continue

        const directionDot = (vx * dx + vy * dy) / (Math.hypot(vx, vy) * Math.max(distance, 1))
        if (directionDot < 0.75) continue

        const eta = distance / speed
        if (eta <= MAX_ETA_MS) {
          prefetchUrl(link.getAttribute('href'))
        }
      }
    }

    const onPointerOver = (event) => {
      const anchor = event.target instanceof Element ? event.target.closest('a[data-predictive="true"]') : null
      if (!anchor) return
      prefetchUrl(anchor.getAttribute('href'))
    }

    window.addEventListener('pointermove', onPointerMove, { passive: true })
    window.addEventListener('pointerover', onPointerOver, { passive: true })
    return () => {
      window.removeEventListener('pointermove', onPointerMove)
      window.removeEventListener('pointerover', onPointerOver)
    }
  }, [router])

  return children
}
