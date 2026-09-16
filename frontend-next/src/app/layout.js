import './globals.css'
import Navbar from '@/components/Navbar'
import Footer from '@/components/Footer'
import ServiceWorkerRegistration from '@/components/ServiceWorkerRegistration'
import PredictivePrefetchProvider from '@/components/PredictivePrefetchProvider'
import LedgerHeartbeat from '@/components/LedgerHeartbeat'
import { AppProvider } from '@/context/AppContext'
import WalletModal from '@/components/WalletModal'
import CreateProjectModal from '@/components/CreateProjectModal'
import ToastContainer from '@/components/ToastContainer'

export const metadata = {
  title: 'PIFP Stellar - Public Impact Funding Protocol',
  description: 'Verifiable milestone-based crowdfunding secured by Soroban smart contracts on the Stellar network.'
}

export default function RootLayout({ children }) {
  return (
    <html lang="en">
      <body>
        <AppProvider>
          <ServiceWorkerRegistration />
          <LedgerHeartbeat />
          <PredictivePrefetchProvider>
            <Navbar />
            <main>
              {children}
            </main>
            <Footer />
            <WalletModal />
            <CreateProjectModal />
            <ToastContainer />
          </PredictivePrefetchProvider>
        </AppProvider>
      </body>
    </html>
  )
}
