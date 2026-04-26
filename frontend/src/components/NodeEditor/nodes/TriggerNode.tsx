import React from 'react';
import { Handle, Position } from '@xyflow/react';

export default function TriggerNode({ data, isConnectable }: any) {
  return (
    <div className="custom-node trigger">
      <div className="custom-node-header">Trigger</div>
      <div>
        <label style={{ fontSize: '0.8rem', display: 'block' }}>Event Action:</label>
        <select
          defaultValue={data.action || 'Price Change'}
          onChange={(evt) => data.onChange?.(evt.target.value, 'action')}
          style={{ width: '100%', marginTop: '5px', padding: '4px', background: '#333', color: 'white', border: 'none', borderRadius: '4px' }}
        >
          <option value="Price Change">Price Change</option>
          <option value="Deposit Received">Deposit Received</option>
          <option value="Time Elapsed">Time Elapsed</option>
        </select>
      </div>
      {/* Trigger nodes only have outputs */}
      <Handle
        type="source"
        position={Position.Bottom}
        id="default"
        isConnectable={isConnectable}
        style={{ background: '#4caf50' }}
      />
    </div>
  );
}
