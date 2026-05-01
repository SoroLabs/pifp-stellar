# frontend-next

Next.js App Router implementation with:

- Predictive client-side prefetching based on pointer velocity + trajectory
- Custom service worker interception/caching for route payloads and JSON requests
- Ledger-close staleness invalidation with a 5-second freshness envelope

## Run

```bash
npm install
npm run dev
```

Open `http://localhost:3000`.

## Implementation overview

- Predictive engine: `src/components/PredictivePrefetchProvider.jsx`
- Service worker registration: `src/components/ServiceWorkerRegistration.jsx`
- Ledger freshness polling: `src/components/LedgerHeartbeat.jsx`
- SW logic: `public/prefetch-sw.js`
- Ledger API source: `src/app/api/ledger/latest/route.js`
