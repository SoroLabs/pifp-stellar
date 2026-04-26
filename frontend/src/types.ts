export type SignalingMessage = 
  | { type: 'join', roomId: String }
  | { type: 'signal', roomId: String, data: any };

export type P2PMessage = 
  | { type: 'chat', text: string, sender: string }
  | { type: 'order_update', price: number, amount: number, side: 'buy' | 'sell' }
  | { type: 'transaction_proposal', xdr: string }
  | { type: 'transaction_signature', signature: string };
