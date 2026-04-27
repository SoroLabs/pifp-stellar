import { useState, useEffect } from 'react'

const IPFS_GATEWAY = 'https://ipfs.io/ipfs/'

export function IpfsResolver({ uri, alt = "IPFS Content" }) {
  const [url, setUrl] = useState('')
  const [isLoading, setIsLoading] = useState(true)
  const [error, setError] = useState(false)

  useEffect(() => {
    if (!uri) return
    
    // Resolve ipfs:// protocol to gateway URL
    const cid = uri.replace('ipfs://', '')
    const resolvedUrl = `${IPFS_GATEWAY}${cid}`
    
    setUrl(resolvedUrl)
    setIsLoading(true)
    setError(false)
  }, [uri])

  return (
    <div className="ipfs-media">
      {isLoading && <div className="media-loader">Loading from IPFS...</div>}
      <img
        src={url}
        alt={alt}
        onLoad={() => setIsLoading(false)}
        onError={() => {
          setIsLoading(false)
          setError(true)
        }}
        style={{ display: isLoading || error ? 'none' : 'block', maxWidth: '100%' }}
      />
      {error && <div className="media-error">Failed to resolve IPFS content</div>}
    </div>
  )
}
