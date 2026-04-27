import React from 'react';
import { Handle, Position } from '@xyflow/react';

export default function ActionNode({ data, isConnectable }: any) {
  return (
    <div className="custom-node action">
      <div className="custom-node-header">Action</div>
      
      <Handle
        type="target"
        position={Position.Top}
        isConnectable={isConnectable}
        style={{ background: '#2196f3' }}
      />

      <div style={{ marginTop: '10px' }}>
        <select
          defaultValue={data.actionType || 'Swap'}
          onChange={(evt) => data.onChange?.(evt.target.value, 'actionType')}
          style={{ width: '100%', marginBottom: '5px', padding: '4px', background: '#333', color: 'white', border: 'none', borderRadius: '4px' }}
        >
          <option value="Swap">Swap</option>
          <option value="Stake">Stake</option>
          <option value="Transfer">Transfer</option>
        </select>
        
        <input 
          type="text" 
          placeholder="Target (e.g. USDC)"
          defaultValue={data.target || ''} 
          onChange={(evt) => data.onChange?.(evt.target.value, 'target')}
          style={{ width: '100%', boxSizing: 'border-box', padding: '4px', background: '#333', color: 'white', border: 'none', borderRadius: '4px' }}
        />
      </div>

      <Handle
        type="source"
        position={Position.Bottom}
        id="default"
        isConnectable={isConnectable}
        style={{ background: '#2196f3' }}
      />
    </div>
  );
}
