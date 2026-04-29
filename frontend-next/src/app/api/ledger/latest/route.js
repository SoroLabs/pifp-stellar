import { NextResponse } from 'next/server'

let simulatedLedger = 100000
let latestLedgerCloseMs = Date.now()

export async function GET() {
  const now = Date.now()
  const elapsed = now - latestLedgerCloseMs
  if (elapsed >= 5000) {
    const increments = Math.max(1, Math.floor(elapsed / 5000))
    simulatedLedger += increments
    latestLedgerCloseMs += increments * 5000
  }

  return NextResponse.json(
    {
      latestLedger: simulatedLedger,
      latestLedgerCloseMs
    },
    {
      headers: {
        'cache-control': 'no-store',
        'x-ledger-close-ms': String(latestLedgerCloseMs)
      }
    }
  )
}
