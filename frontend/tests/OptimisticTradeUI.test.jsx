import { render, screen } from '@testing-library/react'
import { OptimisticTradeUI } from '../src/components/OptimisticTradeUI'

test('renders optimistic trade UI', () => {
  render(<OptimisticTradeUI />)
  expect(screen.getByText('Optimistic P2P Trade')).toBeInTheDocument()
})
