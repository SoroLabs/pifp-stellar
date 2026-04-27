import { useEffect, useRef } from 'react'

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
