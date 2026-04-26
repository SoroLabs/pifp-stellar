import { Node, Edge } from '@xyflow/react';

export function compileToXDR(nodes: Node[], edges: Edge[]) {
  // Validate trigger node existence
  const triggerNode = nodes.find((n) => n.type === 'trigger');
  if (!triggerNode) {
    throw new Error('Workflow must start with a Trigger Node.');
  }

  // Build an adjacency list for traversal
  const adjList = new Map<string, { target: string; handle: string }[]>();
  edges.forEach((edge) => {
    if (!adjList.has(edge.source)) {
      adjList.set(edge.source, []);
    }
    adjList.get(edge.source)?.push({
      target: edge.target,
      handle: edge.sourceHandle || 'default'
    });
  });

  // Visited set to prevent infinite loops
  const visited = new Set<string>();

  // Recursive traversal function
  const traverse = (nodeId: string): any => {
    if (visited.has(nodeId)) {
      throw new Error(`Cycle detected at node ${nodeId}. Workflows must be acyclic.`);
    }
    visited.add(nodeId);

    const node = nodes.find((n) => n.id === nodeId);
    if (!node) return null;

    const children = adjList.get(nodeId) || [];
    
    // Construct the AST representation based on node type
    switch (node.type) {
      case 'trigger': {
        const nextEdge = children[0];
        return {
          type: 'Trigger',
          action: node.data.action || 'On Event',
          next: nextEdge ? traverse(nextEdge.target) : null
        };
      }
      case 'condition': {
        const trueEdge = children.find(c => c.handle === 'true');
        const falseEdge = children.find(c => c.handle === 'false');
        return {
          type: 'Condition',
          operator: node.data.operator || '>',
          value: node.data.value || 0,
          onTrue: trueEdge ? traverse(trueEdge.target) : null,
          onFalse: falseEdge ? traverse(falseEdge.target) : null
        };
      }
      case 'action': {
        const nextEdge = children[0];
        return {
          type: 'Action',
          actionType: node.data.actionType || 'Swap',
          target: node.data.target || 'Token',
          next: nextEdge ? traverse(nextEdge.target) : null
        };
      }
      default:
        return {
          type: 'Unknown',
          id: nodeId
        };
    }
  };

  // Traverse starting from the trigger
  const ast = traverse(triggerNode.id);
  
  // Return the compiled JSON AST format simulating a Soroban payload
  return {
    version: '1.0',
    type: 'SorobanXDRPayload',
    ast
  };
}
