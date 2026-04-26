import React, { useState, useRef, useCallback } from 'react';
import {
  ReactFlow,
  ReactFlowProvider,
  addEdge,
  useNodesState,
  useEdgesState,
  Controls,
  Background,
  Connection,
  Edge,
} from '@xyflow/react';
import '@xyflow/react/dist/style.css';
import { v4 as uuidv4 } from 'uuid';

import TriggerNode from './nodes/TriggerNode';
import ConditionNode from './nodes/ConditionNode';
import ActionNode from './nodes/ActionNode';
import Sidebar from './Sidebar';
import { compileToXDR } from './compiler';
import './NodeEditor.css';

const nodeTypes = {
  trigger: TriggerNode,
  condition: ConditionNode,
  action: ActionNode,
};

let id = 0;
const getId = () => `dndnode_${id++}`;

export default function NodeEditor() {
  const reactFlowWrapper = useRef<HTMLDivElement>(null);
  const [nodes, setNodes, onNodesChange] = useNodesState([]);
  const [edges, setEdges, onEdgesChange] = useEdgesState([]);
  const [reactFlowInstance, setReactFlowInstance] = useState<any>(null);
  const [compiledAST, setCompiledAST] = useState<string>('');

  const onConnect = useCallback(
    (params: Connection | Edge) => {
      // Basic strict runtime type checking based on node types
      const sourceNode = nodes.find(n => n.id === params.source);
      const targetNode = nodes.find(n => n.id === params.target);
      
      if (!sourceNode || !targetNode) return;

      // Rule: Trigger cannot connect to Trigger
      if (sourceNode.type === 'trigger' && targetNode.type === 'trigger') {
        alert('Cannot connect a trigger to another trigger.');
        return;
      }

      setEdges((eds) => addEdge(params, eds));
    },
    [nodes, setEdges],
  );

  const onDragOver = useCallback((event: React.DragEvent) => {
    event.preventDefault();
    event.dataTransfer.dropEffect = 'move';
  }, []);

  const onDrop = useCallback(
    (event: React.DragEvent) => {
      event.preventDefault();

      if (!reactFlowInstance) {
        return;
      }

      const type = event.dataTransfer.getData('application/reactflow');

      if (typeof type === 'undefined' || !type) {
        return;
      }

      const position = reactFlowInstance.screenToFlowPosition({
        x: event.clientX,
        y: event.clientY,
      });

      const newNode = {
        id: getId(),
        type,
        position,
        data: {
          onChange: (val: any, field: string) => {
            setNodes((nds) =>
              nds.map((node) => {
                if (node.id === newNode.id) {
                  node.data = {
                    ...node.data,
                    [field]: val,
                  };
                }
                return node;
              })
            );
          }
        },
      };

      setNodes((nds) => nds.concat(newNode));
    },
    [reactFlowInstance, setNodes],
  );

  const handleCompile = () => {
    try {
      const ast = compileToXDR(nodes, edges);
      setCompiledAST(JSON.stringify(ast, null, 2));
    } catch (err: any) {
      alert(err.message || 'Error compiling graph.');
    }
  };

  return (
    <div>
      <div className="node-editor-container">
        <ReactFlowProvider>
          <Sidebar />
          <div className="react-flow-wrapper" ref={reactFlowWrapper}>
            <ReactFlow
              nodes={nodes}
              edges={edges}
              onNodesChange={onNodesChange}
              onEdgesChange={onEdgesChange}
              onConnect={onConnect}
              onInit={setReactFlowInstance}
              onDrop={onDrop}
              onDragOver={onDragOver}
              nodeTypes={nodeTypes}
              fitView
              style={{ background: '#121212' }}
            >
              <Controls />
              <Background color="#333" gap={16} />
            </ReactFlow>
          </div>
        </ReactFlowProvider>
      </div>

      <div className="compiler-panel">
        <h3>AST Compiler</h3>
        <button className="compile-button" onClick={handleCompile}>
          Compile to XDR Payload
        </button>
        {compiledAST && (
          <pre>{compiledAST}</pre>
        )}
      </div>
    </div>
  );
}
