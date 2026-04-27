import { useState } from 'react'

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
