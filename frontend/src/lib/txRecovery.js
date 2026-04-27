const DEFAULT_ORACLE_URL = (import.meta.env.VITE_ORACLE_DIAGNOSTICS_API_URL || 'http://localhost:9090').replace(/\/$/, '')

const TX_HASH_REGEX = /\b[a-fA-F0-9]{64}\b/

export function parseTxHashFromMessage(message) {
  if (!message) return null
  const match = String(message).match(TX_HASH_REGEX)
  return match ? match[0] : null
}

export function isLikelyTransactionError(message) {
  if (!message) return false
  const normalized = String(message).toLowerCase()
  return (
    normalized.includes('transaction') ||
    normalized.includes('tx ') ||
    normalized.includes('contract error') ||
    normalized.includes('soroban')
  )
}

export async function fetchTxDiagnostics(hash) {
  const response = await fetch(`${DEFAULT_ORACLE_URL}/api/v1/tx/diagnostics/${hash}`)
  if (!response.ok) {
    const payload = await response.json().catch(() => ({}))
    const message = payload.error || `Diagnostics endpoint returned ${response.status}`
    throw new Error(message)
  }
  return response.json()
}
