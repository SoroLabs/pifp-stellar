use std::collections::{BTreeMap, BTreeSet};

use crate::error::{DagResolutionError, Result};
use crate::model::{DependencyEdge, DependencyReason, Intent};

#[derive(Debug, Clone)]
pub struct DependencyGraph {
    pub intents: Vec<Intent>,
    pub edges: Vec<DependencyEdge>,
    adjacency: Vec<Vec<usize>>,
    indegree: Vec<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mark {
    White,
    Gray,
    Black,
}

impl DependencyGraph {
    pub fn build(intents: Vec<Intent>) -> Result<Self> {
        if intents.is_empty() {
            return Err(DagResolutionError::EmptyInput);
        }

        let mut normalized = Vec::with_capacity(intents.len());
        let mut seen_ids = BTreeSet::new();
        for intent in intents {
            let id = intent.id.trim().to_string();
            if id.is_empty() {
                return Err(DagResolutionError::InvalidIntent {
                    id: intent.id,
                    reason: "intent id cannot be empty".to_string(),
                });
            }
            if !seen_ids.insert(id.clone()) {
                return Err(DagResolutionError::DuplicateIntentId(id));
            }
            let reads = normalize_resources(&intent.reads);
            let writes = normalize_resources(&intent.writes);
            let after = normalize_resources(&intent.after);
            normalized.push(Intent {
                id,
                reads,
                writes,
                after,
            });
        }

        let mut id_to_index = BTreeMap::new();
        for (index, intent) in normalized.iter().enumerate() {
            id_to_index.insert(intent.id.clone(), index);
        }

        let mut edge_map: BTreeMap<(usize, usize), Vec<DependencyReason>> = BTreeMap::new();
        let mut last_writer: BTreeMap<String, usize> = BTreeMap::new();
        let mut open_readers: BTreeMap<String, BTreeSet<usize>> = BTreeMap::new();

        for (index, intent) in normalized.iter().enumerate() {
            for dependency in &intent.after {
                let Some(&dependency_index) = id_to_index.get(dependency) else {
                    return Err(DagResolutionError::UnknownDependency {
                        intent: intent.id.clone(),
                        dependency: dependency.clone(),
                    });
                };
                if dependency_index == index {
                    return Err(DagResolutionError::InvalidIntent {
                        id: intent.id.clone(),
                        reason: "an intent cannot depend on itself".to_string(),
                    });
                }
                add_reason(
                    &mut edge_map,
                    dependency_index,
                    index,
                    DependencyReason::ExplicitAfter {
                        dependency: dependency.clone(),
                    },
                );
            }

            for resource in &intent.reads {
                if let Some(&writer) = last_writer.get(resource) {
                    add_reason(
                        &mut edge_map,
                        writer,
                        index,
                        DependencyReason::ReadAfterWrite {
                            resource: resource.clone(),
                        },
                    );
                }
                open_readers
                    .entry(resource.clone())
                    .or_default()
                    .insert(index);
            }

            for resource in &intent.writes {
                if let Some(&writer) = last_writer.get(resource) {
                    add_reason(
                        &mut edge_map,
                        writer,
                        index,
                        DependencyReason::WriteAfterWrite {
                            resource: resource.clone(),
                        },
                    );
                }

                if let Some(readers) = open_readers.get(resource) {
                    for reader in readers {
                        if *reader == index {
                            continue;
                        }
                        add_reason(
                            &mut edge_map,
                            *reader,
                            index,
                            DependencyReason::WriteAfterRead {
                                resource: resource.clone(),
                            },
                        );
                    }
                }

                last_writer.insert(resource.clone(), index);
                open_readers.insert(resource.clone(), BTreeSet::new());
            }
        }

        let mut edges = Vec::with_capacity(edge_map.len());
        let mut adjacency = vec![Vec::new(); normalized.len()];
        let mut indegree = vec![0usize; normalized.len()];

        for ((from, to), reasons) in edge_map {
            adjacency[from].push(to);
            indegree[to] += 1;
            let mut reasons = reasons;
            reasons.sort();
            reasons.dedup();
            edges.push(DependencyEdge {
                from: normalized[from].id.clone(),
                to: normalized[to].id.clone(),
                reasons,
            });
        }

        for neighbors in &mut adjacency {
            neighbors.sort_unstable();
            neighbors.dedup();
        }

        Ok(Self {
            intents: normalized,
            edges,
            adjacency,
            indegree,
        })
    }

    pub fn topological_layers(&self) -> Result<Vec<Vec<usize>>> {
        let mut indegree = self.indegree.clone();
        let mut ready: BTreeSet<usize> = indegree
            .iter()
            .enumerate()
            .filter_map(|(index, degree)| (*degree == 0).then_some(index))
            .collect();
        let mut layers = Vec::new();
        let mut processed = 0usize;

        while !ready.is_empty() {
            let layer: Vec<usize> = ready.iter().copied().collect();
            ready.clear();

            for &index in &layer {
                processed += 1;
                for &neighbor in &self.adjacency[index] {
                    indegree[neighbor] = indegree[neighbor].saturating_sub(1);
                    if indegree[neighbor] == 0 {
                        ready.insert(neighbor);
                    }
                }
            }

            layers.push(layer);
        }

        if processed != self.intents.len() {
            let remaining: BTreeSet<usize> = indegree
                .iter()
                .enumerate()
                .filter_map(|(index, degree)| (*degree > 0).then_some(index))
                .collect();
            let cycle = self.find_cycle(&remaining).unwrap_or_else(|| {
                remaining
                    .iter()
                    .map(|index| self.intents[*index].id.clone())
                    .collect()
            });
            return Err(DagResolutionError::CycleDetected(cycle.join(" -> ")));
        }

        Ok(layers)
    }

    fn find_cycle(&self, remaining: &BTreeSet<usize>) -> Option<Vec<String>> {
        let mut marks = vec![Mark::White; self.intents.len()];
        let mut stack = Vec::new();

        for &start in remaining {
            if marks[start] == Mark::White {
                if let Some(cycle) = self.dfs_cycle(start, remaining, &mut marks, &mut stack) {
                    return Some(cycle);
                }
            }
        }

        None
    }

    fn dfs_cycle(
        &self,
        index: usize,
        remaining: &BTreeSet<usize>,
        marks: &mut [Mark],
        stack: &mut Vec<usize>,
    ) -> Option<Vec<String>> {
        marks[index] = Mark::Gray;
        stack.push(index);

        for &neighbor in &self.adjacency[index] {
            if !remaining.contains(&neighbor) {
                continue;
            }
            match marks[neighbor] {
                Mark::White => {
                    if let Some(cycle) = self.dfs_cycle(neighbor, remaining, marks, stack) {
                        return Some(cycle);
                    }
                }
                Mark::Gray => {
                    let start = stack.iter().position(|current| *current == neighbor)?;
                    let mut cycle = stack[start..]
                        .iter()
                        .map(|current| self.intents[*current].id.clone())
                        .collect::<Vec<_>>();
                    cycle.push(self.intents[neighbor].id.clone());
                    return Some(cycle);
                }
                Mark::Black => {}
            }
        }

        stack.pop();
        marks[index] = Mark::Black;
        None
    }
}

fn normalize_resources(values: &[String]) -> Vec<String> {
    values
        .iter()
        .filter_map(|value| {
            let trimmed = value.trim();
            (!trimmed.is_empty()).then_some(trimmed.to_string())
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn add_reason(
    edge_map: &mut BTreeMap<(usize, usize), Vec<DependencyReason>>,
    from: usize,
    to: usize,
    reason: DependencyReason,
) {
    edge_map.entry((from, to)).or_default().push(reason);
}
