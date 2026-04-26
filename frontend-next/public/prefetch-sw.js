const DYNAMIC_CACHE = 'predictive-dynamic-v1'
const LEDGER_STALE_MS = 5000

let latestLedgerCloseMs = 0
const cacheMeta = new Map()

self.addEventListener('install', () => {
  self.skipWaiting()
})

self.addEventListener('activate', (event) => {
  event.waitUntil(self.clients.claim())
})

self.addEventListener('message', (event) => {
  const data = event.data || {}
  if (data.type === 'LEDGER_UPDATE' && Number.isFinite(data.latestLedgerCloseMs)) {
    latestLedgerCloseMs = data.latestLedgerCloseMs
    event.waitUntil(invalidateStale())
    return
  }

  if (data.type === 'PREFETCH_URL' && typeof data.href === 'string') {
    const url = new URL(data.href, self.location.origin)
    if (url.origin === self.location.origin) {
      event.waitUntil(prefetchAndCache(url.toString()))
    }
  }
})

self.addEventListener('fetch', (event) => {
  const { request } = event
  if (request.method !== 'GET') return

  const url = new URL(request.url)
  if (url.origin !== self.location.origin) return
  if (!isCacheTarget(request, url)) return

  event.respondWith(cacheFirstWithLedgerGuard(request))
})

function isCacheTarget(request, url) {
  if (url.pathname.startsWith('/api/ledger/latest')) return false
  if (url.pathname.startsWith('/_next/static/')) return false
  if (url.pathname.startsWith('/projects/')) return true
  if (url.pathname.startsWith('/api/')) return true
  if (url.searchParams.has('__flight__')) return true
  return request.headers.has('rsc') || request.headers.get('accept')?.includes('text/x-component')
}

async function cacheFirstWithLedgerGuard(request) {
  const cache = await caches.open(DYNAMIC_CACHE)
  const key = request.url
  const cached = await cache.match(request)
  const now = Date.now()

  if (cached) {
    const fetchedAt = cacheMeta.get(key) ?? 0
    const tooOld = now - fetchedAt > LEDGER_STALE_MS
    const behindLedger = latestLedgerCloseMs > 0 && fetchedAt < latestLedgerCloseMs
    if (!tooOld && !behindLedger) {
      return cached
    }
  }

  try {
    const network = await fetch(request, { cache: 'no-store' })
    if (network.ok) {
      await cache.put(request, network.clone())
      cacheMeta.set(key, now)
    }
    return network
  } catch {
    if (cached) return cached
    throw new Error('Network unavailable and cache miss')
  }
}

async function prefetchAndCache(href) {
  const cache = await caches.open(DYNAMIC_CACHE)
  const request = new Request(href, { method: 'GET' })
  const response = await fetch(request, {
    headers: {
      'x-prefetch-intent': 'predictive'
    }
  })
  if (response.ok) {
    await cache.put(request, response.clone())
    cacheMeta.set(href, Date.now())
  }
}

async function invalidateStale() {
  const cache = await caches.open(DYNAMIC_CACHE)
  const keys = await cache.keys()
  const now = Date.now()

  await Promise.all(
    keys.map(async (request) => {
      const fetchedAt = cacheMeta.get(request.url) ?? 0
      const tooOld = now - fetchedAt > LEDGER_STALE_MS
      const behindLedger = latestLedgerCloseMs > 0 && fetchedAt < latestLedgerCloseMs
      if (tooOld || behindLedger) {
        await cache.delete(request)
        cacheMeta.delete(request.url)
      }
    })
  )
}
