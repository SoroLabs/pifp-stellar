import React, { useState, useEffect, useCallback } from 'react';
import {
  ReactFlow,
  useNodesState,
  useEdgesState,
  addEdge,
  MarkerType,
  Background,
  Controls,
} from '@xyflow/react';
import '@xyflow/react/dist/style.css';

const ORACLE_API = 'http://localhost:9090/api/debt/optimize';

const initialEdges = [
  { from: 'Alice', to: 'Bob', amount: 100 },
  { from: 'Bob', to: 'Charlie', amount: 100 },
  { from: 'Charlie', to: 'Alice', amount: 100 },
  { from: 'David', to: 'Alice', amount: 50 },
  { from: 'Charlie', to: 'David', amount: 30 },
];

const DebtVisualizer = () => {
  const [nodes, setNodes, onNodesChange] = useNodesState([]);
  const [edges, setEdges, onEdgesChange] = useEdgesState([]);
  const [isOptimized, setIsOptimized] = useState(false);
  const [originalData, setOriginalData] = useState(null);
  const [optimizedData, setOptimizedData] = useState(null);
  const [isLoading, setIsLoading] = useState(false);

  const convertToFlowData = (edgeData) => {
    const uniqueNodes = Array.from(new Set(edgeData.flatMap(e => [e.from, e.to])));
    
    const flowNodes = uniqueNodes.map((node, i) => ({
      id: node,
      data: { label: node },
      position: { x: 250 + 200 * Math.cos((2 * Math.PI * i) / uniqueNodes.length), y: 250 + 200 * Math.sin((2 * Math.PI * i) / uniqueNodes.length) },
      style: { background: '#1e293b', color: '#f8fafc', border: '1px solid #3b82f6', borderRadius: '8px', padding: '10px' },
    }));

    const flowEdges = edgeData.map((e, i) => ({
      id: `e${i}-${e.from}-${e.to}`,
      source: e.from,
      target: e.to,
      label: `$${e.amount}`,
      animated: true,
      labelStyle: { fill: '#3b82f6', fontWeight: 700 },
      style: { stroke: '#3b82f6', strokeWidth: 2 },
      markerEnd: {
        type: MarkerType.ArrowClosed,
        color: '#3b82f6',
      },
    }));

    return { flowNodes, flowEdges };
  };

  const fetchOptimization = async () => {
    setIsLoading(true);
    try {
      const response = await fetch(ORACLE_API, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ edges: initialEdges }),
      });
      const data = await response.json();
      setOriginalData(data.original);
      setOptimizedData(data.optimized);
      
      const { flowNodes, flowEdges } = convertToFlowData(data.original);
      setNodes(flowNodes);
      setEdges(flowEdges);
    } catch (err) {
      console.error('Failed to fetch debt optimization:', err);
    } finally {
      setIsLoading(false);
    }
  };

  useEffect(() => {
    fetchOptimization();
  }, []);

  const toggleOptimization = () => {
    const data = isOptimized ? originalData : optimizedData;
    const { flowNodes, flowEdges } = convertToFlowData(data);
    
    // Smoothly update nodes and edges
    setNodes(flowNodes);
    setEdges(flowEdges);
    setIsOptimized(!isOptimized);
  };

  return (
    <div className="debt-visualizer-container">
      <div className="controls-bar">
        <h3>Debt Minimization Router</h3>
        <div className="actions">
          <button 
            className={`toggle-btn ${isOptimized ? 'optimized' : ''}`}
            onClick={toggleOptimization}
            disabled={isLoading}
          >
            {isOptimized ? 'Show Original Debt' : 'Optimize Debt Cycles'}
          </button>
        </div>
      </div>

      <div style={{ width: '100%', height: '500px', background: '#0f172a', borderRadius: '12px', border: '1px solid #1e293b' }}>
        <ReactFlow
          nodes={nodes}
          edges={edges}
          onNodesChange={onNodesChange}
          onEdgesChange={onEdgesChange}
          fitView
        >
          <Background color="#1e293b" gap={20} />
          <Controls />
        </ReactFlow>
      </div>

      <div className="legend">
        <div className="info-card">
          <h4>{isOptimized ? 'Optimized State' : 'Current Obligations'}</h4>
          <p>
            {isOptimized 
              ? 'Cycles detected and cancelled. Total outstanding debt reduced.' 
              : 'Complex web of IOUs with potential circular dependencies.'}
          </p>
        </div>
      </div>

      <style jsx>{`
        .debt-visualizer-container {
          padding: 20px;
          display: flex;
          flex-direction: column;
          gap: 20px;
        }
        .controls-bar {
          display: flex;
          justify-content: space-between;
          align-items: center;
        }
        .controls-bar h3 {
          margin: 0;
          color: #f8fafc;
        }
        .toggle-btn {
          background: #3b82f6;
          color: white;
          border: none;
          padding: 10px 20px;
          border-radius: 8px;
          font-weight: 600;
          cursor: pointer;
          transition: all 0.3s ease;
        }
        .toggle-btn:hover {
          background: #2563eb;
          transform: translateY(-2px);
        }
        .toggle-btn.optimized {
          background: #10b981;
        }
        .legend {
          display: flex;
          gap: 20px;
        }
        .info-card {
          background: #1e293b;
          padding: 15px;
          border-radius: 12px;
          border: 1px solid #3b82f6;
          flex: 1;
        }
        .info-card h4 {
          margin: 0 0 10px 0;
          color: #3b82f6;
        }
        .info-card p {
          margin: 0;
          color: #94a3b8;
          font-size: 0.9rem;
        }
      `}</style>
    </div>
  );
};

export default DebtVisualizer;
