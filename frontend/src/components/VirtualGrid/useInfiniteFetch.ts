import { useState, useEffect, useCallback, useRef } from 'react';

export interface Transaction {
  id: string;
  hash: string;
  ledger: number;
  fee: string;
  status: 'success' | 'failed';
  time: string;
}

export function useInfiniteFetch(batchSize: number = 100) {
  const [items, setItems] = useState<Transaction[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [hasMore, setHasMore] = useState(true);
  const cursorRef = useRef(0);

  const fetchMore = useCallback(async () => {
    if (isLoading || !hasMore) return;
    setIsLoading(true);

    // Simulate network delay and cursor-based fetching
    setTimeout(() => {
      const newItems: Transaction[] = [];
      const currentCursor = cursorRef.current;
      
      for (let i = 0; i < batchSize; i++) {
        const idNum = currentCursor + i;
        newItems.push({
          id: `tx_${idNum}`,
          hash: `0x${Math.random().toString(16).substring(2, 10).padStart(8, '0')}...${Math.random().toString(16).substring(2, 6)}`,
          ledger: 50000000 + idNum,
          fee: (Math.random() * 0.05).toFixed(5),
          status: Math.random() > 0.05 ? 'success' : 'failed',
          time: new Date(Date.now() - idNum * 1000).toLocaleTimeString()
        });
      }

      setItems(prev => [...prev, ...newItems]);
      cursorRef.current += batchSize;
      setIsLoading(false);
      
      // Cap at 100k records for demonstration
      if (cursorRef.current >= 100000) {
        setHasMore(false);
      }
    }, 300);
  }, [isLoading, hasMore, batchSize]);

  // Initial fetch
  useEffect(() => {
    fetchMore();
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  return { items, isLoading, hasMore, fetchMore };
}
