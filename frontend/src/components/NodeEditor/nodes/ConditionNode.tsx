import React from 'react';
import { Handle, Position } from '@xyflow/react';

export default function ConditionNode({ data, isConnectable }: any) {
  return (
    <div className="custom-node condition">
      <div className="custom-node-header">Condition</div>
      
      <Handle
        type="target"
        position={Position.Top}
        isConnectable={isConnectable}
        style={{ background: '#ff9800' }}
      />

      <div style={{ display: 'flex', gap: '5px', marginTop: '10px' }}>
        <select
          defaultValue={data.operator || '>'}
          onChange={(evt) => data.onChange?.(evt.target.value, 'operator')}
          style={{ width: '50%', padding: '4px', background: '#333', color: 'white', border: 'none', borderRadius: '4px' }}
        >
          <option value=">">&gt;</option>
          <option value="<">&lt;</option>
          <option value="==">==</option>
          <option value="!=">!=</option>
        </select>
        <input 
          type="number" 
          defaultValue={data.value || 0} 
          onChange={(evt) => data.onChange?.(evt.target.value, 'value')}
          style={{ width: '50%', padding: '4px', background: '#333', color: 'white', border: 'none', borderRadius: '4px' }}
        />
      </div>

      <div style={{ marginTop: '15px', fontSize: '0.8rem', display: 'flex', justifyContent: 'space-between' }}>
        <span>True</span>
        <span>False</span>
      </div>

      <Handle
        type="source"
        position={Position.Bottom}
        id="true"
        style={{ left: '30%', background: '#4caf50' }}
        isConnectable={isConnectable}
      />
      <Handle
        type="source"
        position={Position.Bottom}
        id="false"
        style={{ left: '70%', background: '#f44336' }}
        isConnectable={isConnectable}
      />
    </div>
  );
}
