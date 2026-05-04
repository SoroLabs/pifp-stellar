import { useState } from 'react'

const IPFS_GATEWAY = 'https://ipfs.io/ipfs/'

function ResolvedImage({ url, alt }) {
  const [isLoading, setIsLoading] = useState(true)
  const [error, setError] = useState(false)

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

export function IpfsResolver({ uri, alt = "IPFS Content" }) {
  if (!uri) {
    return null
  }

  const cid = uri.replace('ipfs://', '')
  const resolvedUrl = `${IPFS_GATEWAY}${cid}`

  return <ResolvedImage key={resolvedUrl} url={resolvedUrl} alt={alt} />
}
