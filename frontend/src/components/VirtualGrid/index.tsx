import React, { useState, useEffect, useRef, useCallback } from 'react';
import { MeasurementEngine } from './MeasurementEngine';
import { useInfiniteFetch } from './useInfiniteFetch';
import { TransactionRow } from './TransactionRow';
import './VirtualGrid.css';

const DEFAULT_ROW_HEIGHT = 60;
const OVERSCAN = 5; // Number of items to render outside the viewport

export default function VirtualGrid() {
  const { items, isLoading, hasMore, fetchMore } = useInfiniteFetch(100);
  const containerRef = useRef<HTMLDivElement>(null);
  
  // Singleton MeasurementEngine per component mount
  const engineRef = useRef(new MeasurementEngine(DEFAULT_ROW_HEIGHT));
  
  const [scrollTop, setScrollTop] = useState(0);
  const [viewportHeight, setViewportHeight] = useState(600);
  
  // Force a re-render when engine notifies of height changes
  const [, forceUpdate] = useState({});

  useEffect(() => {
    const unsub = engineRef.current.subscribe(() => forceUpdate({}));
    return () => unsub();
  }, []);

  useEffect(() => {
    // We add +1 for the skeleton loader if there is more data
    const totalCount = items.length + (hasMore ? 1 : 0);
    engineRef.current.setItemCount(totalCount);
  }, [items.length, hasMore]);

  const onScroll = useCallback((e: React.UIEvent<HTMLDivElement>) => {
    setScrollTop(e.currentTarget.scrollTop);
    
    // Check if we are near the bottom to trigger fetchMore
    const scrollBottom = e.currentTarget.scrollTop + e.currentTarget.clientHeight;
    const totalHeight = e.currentTarget.scrollHeight;
    
    if (totalHeight - scrollBottom < 500 && hasMore && !isLoading) {
      fetchMore();
    }
  }, [fetchMore, hasMore, isLoading]);

  useEffect(() => {
    if (containerRef.current) {
      setViewportHeight(containerRef.current.clientHeight);
      
      const observer = new ResizeObserver((entries) => {
        for (let entry of entries) {
          setViewportHeight(entry.target.clientHeight);
        }
      });
      observer.observe(containerRef.current);
      return () => observer.disconnect();
    }
  }, []);

  const handleMeasure = useCallback((index: number, height: number) => {
    engineRef.current.measureItem(index, height);
  }, []);

  // Calculate visible window
  const { startIndex, endIndex } = engineRef.current.getVisibleRange(scrollTop, viewportHeight);
  
  const startNode = Math.max(0, startIndex - OVERSCAN);
  const endNode = Math.min(items.length + (hasMore ? 1 : 0) - 1, endIndex + OVERSCAN);

  const visibleRows = [];
  for (let i = startNode; i <= endNode; i++) {
    const isSkeleton = i >= items.length;
    const item = isSkeleton ? null : items[i];
    
    const position = engineRef.current.getPosition(i);
    
    visibleRows.push(
      <TransactionRow
        key={isSkeleton ? 'skeleton' : item!.id}
        index={i}
        item={item}
        onMeasure={handleMeasure}
        style={{
          position: 'absolute',
          top: 0,
          left: 0,
          width: '100%',
          transform: `translateY(${position}px)`,
        }}
      />
    );
  }

  const totalHeight = engineRef.current.getTotalHeight();

  return (
    <div className="virtual-grid-wrapper">
      <div className="grid-header">
        <h3>Block Explorer (Virtual Grid)</h3>
        <span className="stats">
          Loaded: {items.length.toLocaleString()} txs | Virtual Nodes: {visibleRows.length}
        </span>
      </div>
      
      <div 
        className="virtual-grid-container" 
        ref={containerRef}
        onScroll={onScroll}
      >
        <div style={{ height: `${totalHeight}px`, position: 'relative', width: '100%' }}>
          {visibleRows}
        </div>
      </div>
    </div>
  );
}
