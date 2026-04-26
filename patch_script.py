import json
import os

# Path to package.json
package_path = r'c:\Users\DELL\Desktop\Wave 4\pifp-stellar\frontend\package.json'

# Load package.json
with open(package_path, 'r') as f:
    package = json.load(f)

# Add dependencies
new_deps = {
    "xstate": "^5.0.0",
    "helia": "^4.0.0",
    "@helia/http": "^1.0.0",
    "stellar-sdk": "^12.0.0",
    "@ledgerhq/hw-transport-webhid": "^6.27.1",
    "framer-motion": "^11.0.0"
}

package['dependencies'].update(new_deps)

# Write back
with open(package_path, 'w') as f:
    json.dump(package, f, indent=2)

# Component codes
optimistic_code = '''import { useState } from 'react'
import { createMachine, useMachine } from 'xstate'
import { motion } from 'framer-motion'

const tradeMachine = createMachine({
  id: 'trade',
  initial: 'idle',
  states: {
    idle: {
      on: { SUBMIT: 'optimistic' }
    },
    optimistic: {
      on: { CONFIRM: 'confirmed', REJECT: 'rolledBack' }
    },
    confirmed: {
      type: 'final'
    },
    rolledBack: {
      type: 'final'
    }
  }
})

export function OptimisticTradeUI() {
  const [balance, setBalance] = useState(1000)
  const [state, send] = useMachine(tradeMachine)

  const handleSubmit = () => {
    send('SUBMIT')
    setBalance(balance - 100)
    setTimeout(() => {
      if (Math.random() > 0.5) {
        send('CONFIRM')
      } else {
        send('REJECT')
        setBalance(balance + 100)
      }
    }, 2000)
  }

  return (
    <div className="optimistic-trade">
      <h2>Optimistic P2P Trade</h2>
      <p>Balance: <motion.span animate={{ scale: state.matches('optimistic') ? 1.1 : 1, color: state.matches('rolledBack') ? '#ff0000' : '#00ff00' }} transition={{ duration: 0.5 }}>{balance}</motion.span></p>
      <button onClick={handleSubmit} disabled={!state.matches('idle')}>Submit Trade</button>
      <p>State: {state.value}</p>
      {state.matches('rolledBack') && <p className="error">Transaction failed, rolled back</p>}
    </div>
  )
}
'''

ipfs_code = '''import { useState, useRef, useEffect } from 'react'
import { createHelia } from 'helia'
import { http } from '@helia/http'

export function IpfsVideoStreamer({ cid }) {
  const videoRef = useRef()
  const [helia, setHelia] = useState()

  useEffect(() => {
    const initHelia = async () => {
      const h = await createHelia({ transports: [http()] })
      setHelia(h)
    }
    initHelia()
  }, [])

  const loadVideo = async () => {
    if (!helia || !cid) return
    // Simplified: fetch full file
    const response = await fetch(`https://ipfs.io/ipfs/${cid}`)
    const buffer = await response.arrayBuffer()
    const blob = new Blob([buffer], { type: 'video/mp4' })
    videoRef.current.src = URL.createObjectURL(blob)
  }

  useEffect(() => {
    if (helia && cid) {
      loadVideo()
    }
  }, [helia, cid])

  return (
    <div className="ipfs-video">
      <h2>IPFS Video Streaming</h2>
      <video ref={videoRef} controls />
    </div>
  )
}
'''

hw_code = '''import { useState } from 'react'

export function HardwareWalletConnector() {
  const [address, setAddress] = useState('')
  const [error, setError] = useState('')

  const connect = async () => {
    try {
      if (!navigator.hid) {
        throw new Error('WebHID not supported')
      }
      const devices = await navigator.hid.requestDevice({ filters: [{ vendorId: 0x2c97 }] }) // Ledger
      const device = devices[0]
      await device.open()
      // APDU for Stellar address
      const apdu = new Uint8Array([0xe0, 0x02, 0x00, 0x00, 0x00])
      const response = await device.receiveFeatureReport(0)
      // Parse
      setAddress('GABC...') // Mock
    } catch (e) {
      setError(e.message)
    }
  }

  return (
    <div className="hw-wallet">
      <h2>Hardware Wallet Integration</h2>
      <button onClick={connect}>Connect Ledger</button>
      <p>Address: {address}</p>
      {error && <p className="error">{error}</p>}
    </div>
  )
}
'''

iframe_code = '''import { useEffect, useRef } from 'react'

export function SecureIframeEmbed({ src }) {
  const iframeRef = useRef()

  const requestSignature = () => {
    iframeRef.current.contentWindow.postMessage({ type: 'SIGN_REQUEST', data: 'mock_tx' }, src)
  }

  useEffect(() => {
    const handleMessage = (event) => {
      if (event.origin !== new URL(src).origin) return
      console.log('Received:', event.data)
      // Handle signature response
    }
    window.addEventListener('message', handleMessage)
    return () => window.removeEventListener('message', handleMessage)
  }, [src])

  return (
    <div className="secure-iframe">
      <h2>Secure Embedded Dapp</h2>
      <iframe ref={iframeRef} src={src} sandbox="allow-scripts allow-same-origin allow-forms" style={{ width: '100%', height: '400px' }} />
      <button onClick={requestSignature}>Request Signature</button>
    </div>
  )
}
'''

# Create components
os.makedirs('frontend/src/components', exist_ok=True)
with open('frontend/src/components/OptimisticTradeUI.jsx', 'w') as f:
    f.write(optimistic_code)
with open('frontend/src/components/IpfsVideoStreamer.jsx', 'w') as f:
    f.write(ipfs_code)
with open('frontend/src/components/HardwareWalletConnector.jsx', 'w') as f:
    f.write(hw_code)
with open('frontend/src/components/SecureIframeEmbed.jsx', 'w') as f:
    f.write(iframe_code)

# Modify App.jsx
with open('frontend/src/App.jsx', 'r') as f:
    app_content = f.read()

# Add imports
import_block = "import { OptimisticTradeUI } from './components/OptimisticTradeUI'\nimport { IpfsVideoStreamer } from './components/IpfsVideoStreamer'\nimport { HardwareWalletConnector } from './components/HardwareWalletConnector'\nimport { SecureIframeEmbed } from './components/SecureIframeEmbed'\n"
app_content = app_content.replace("import { BridgeWatcher } from './components/BridgeWatcher'\nimport { IpfsUploader } from './components/IpfsUploader'\n", "import { BridgeWatcher } from './components/BridgeWatcher'\nimport { IpfsUploader } from './components/IpfsUploader'\n" + import_block)

# Add tabs
nav_old = '''        <button 
          className={activeTab === 'ipfs' ? 'active' : ''} 
          onClick={() => setActiveTab('ipfs')}
        >
          IPFS Storage
        </button>
      </nav>'''

nav_new = '''        <button 
          className={activeTab === 'ipfs' ? 'active' : ''} 
          onClick={() => setActiveTab('ipfs')}
        >
          IPFS Storage
        </button>
        <button 
          className={activeTab === 'trade' ? 'active' : ''} 
          onClick={() => setActiveTab('trade')}
        >
          Optimistic Trade
        </button>
        <button 
          className={activeTab === 'video' ? 'active' : ''} 
          onClick={() => setActiveTab('video')}
        >
          IPFS Video
        </button>
        <button 
          className={activeTab === 'wallet' ? 'active' : ''} 
          onClick={() => setActiveTab('wallet')}
        >
          Hardware Wallet
        </button>
        <button 
          className={activeTab === 'embed' ? 'active' : ''} 
          onClick={() => setActiveTab('embed')}
        >
          Embedded Dapp
        </button>
      </nav>'''

app_content = app_content.replace(nav_old, nav_new)

# Add sections
sections_old = '''      {activeTab === 'ipfs' && (
        <section className="ipfs">
          <IpfsUploader />
        </section>
      )}'''

sections_new = '''      {activeTab === 'ipfs' && (
        <section className="ipfs">
          <IpfsUploader />
        </section>
      )}

      {activeTab === 'trade' && (
        <section className="trade">
          <OptimisticTradeUI />
        </section>
      )}

      {activeTab === 'video' && (
        <section className="video">
          <IpfsVideoStreamer cid="QmYourCIDHere" />
        </section>
      )}

      {activeTab === 'wallet' && (
        <section className="wallet">
          <HardwareWalletConnector />
        </section>
      )}

      {activeTab === 'embed' && (
        <section className="embed">
          <SecureIframeEmbed src="https://example-dapp.com" />
        </section>
      )}'''

app_content = app_content.replace(sections_old, sections_new)

with open('frontend/src/App.jsx', 'w') as f:
    f.write(app_content)

print("All changes applied")