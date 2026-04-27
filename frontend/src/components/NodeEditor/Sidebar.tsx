import React from 'react';

export default function Sidebar() {
  const onDragStart = (event: React.DragEvent, nodeType: string) => {
    event.dataTransfer.setData('application/reactflow', nodeType);
    event.dataTransfer.effectAllowed = 'move';
  };

  return (
    <aside className="node-editor-sidebar">
      <h3>Nodes</h3>
      <div className="description">You can drag these nodes to the pane on the right.</div>
      
      <div className="dndnode trigger" onDragStart={(event) => onDragStart(event, 'trigger')} draggable>
        Trigger Node
      </div>
      <div className="dndnode condition" onDragStart={(event) => onDragStart(event, 'condition')} draggable>
        Condition Node
      </div>
      <div className="dndnode action" onDragStart={(event) => onDragStart(event, 'action')} draggable>
        Action Node
      </div>
    </aside>
  );
}
