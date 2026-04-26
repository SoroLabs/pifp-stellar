import './globals.css'
import LedgerHeartbeat from '@/components/LedgerHeartbeat'
import PredictivePrefetchProvider from '@/components/PredictivePrefetchProvider'
import ServiceWorkerRegistration from '@/components/ServiceWorkerRegistration'

export const metadata = {
  title: 'PIFP Predictive App Router',
  description: 'Predictive prefetching and RSC cache orchestration demo'
}

export default function RootLayout({ children }) {
  return (
    <html lang="en">
      <body>
        <ServiceWorkerRegistration />
        <LedgerHeartbeat />
        <PredictivePrefetchProvider>{children}</PredictivePrefetchProvider>
      </body>
    </html>
  )
}
