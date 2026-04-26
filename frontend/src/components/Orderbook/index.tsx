import React, { useEffect, useRef, useState } from 'react';
import { OrderbookEngine, OrderUpdate } from './engine';
import './Orderbook.css';

export default function Orderbook() {
  const containerRef = useRef<HTMLTableSectionElement>(null);
  const engineRef = useRef<OrderbookEngine | null>(null);
  const [updatesPerSec, setUpdatesPerSec] = useState(0);

  useEffect(() => {
    if (!containerRef.current) return;

    // Initialize the Differential DOM Updating Engine
    engineRef.current = new OrderbookEngine('orderbook-body');
    
    // Initial mock state
    const initialPrices = Array.from({ length: 20 }, (_, i) => 100 + i * 0.5);
    const initialUpdates = initialPrices.map(p => ({ price: p, size: Math.floor(Math.random() * 100) + 1 }));
    engineRef.current.processUpdates(initialUpdates);

    let updateCount = 0;
    
    // High-frequency mocked event stream (simulate thousands of updates/sec)
    // We'll run a batch of updates every 16ms (~60fps) to process large volumes
    const streamInterval = setInterval(() => {
      const batch: OrderUpdate[] = [];
      const batchSize = 50; // 50 updates * 60fps = 3000 updates/sec
      
      for (let i = 0; i < batchSize; i++) {
        const price = initialPrices[Math.floor(Math.random() * initialPrices.length)];
        const size = Math.random() > 0.1 ? Math.floor(Math.random() * 200) + 1 : 0; // 10% chance to remove
        batch.push({ price, size });
      }

      engineRef.current?.processUpdates(batch);
      updateCount += batchSize;
    }, 16);

    const statsInterval = setInterval(() => {
      setUpdatesPerSec(updateCount);
      updateCount = 0;
    }, 1000);

    return () => {
      clearInterval(streamInterval);
      clearInterval(statsInterval);
      engineRef.current?.destroy();
    };
  }, []);

  return (
    <div className="orderbook-container">
      <h3>Live Orderbook</h3>
      <table className="orderbook-table">
        <thead>
          <tr>
            <th>Price (XLM)</th>
            <th>Size</th>
          </tr>
        </thead>
        {/* React ignores changes inside this tbody due to no state/props mappings. 
            The Differential Engine will directly mutate the DOM here. */}
        <tbody id="orderbook-body" ref={containerRef}></tbody>
      </table>
      <div className="engine-stats">
        <span>Differential Engine: Active</span>
        <span>{updatesPerSec.toLocaleString()} updates/sec</span>
      </div>
    </div>
  );
}
