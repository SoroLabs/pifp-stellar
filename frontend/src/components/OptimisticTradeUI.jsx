import { useState } from 'react'
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
