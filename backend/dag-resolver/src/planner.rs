use crate::error::Result;
use crate::graph::DependencyGraph;
use crate::model::{ParallelBatch, ResolutionReport};

pub fn resolve_intents(intents: Vec<crate::model::Intent>) -> Result<ResolutionReport> {
    let graph = DependencyGraph::build(intents)?;
    let layers = graph.topological_layers()?;

    let batches = layers
        .into_iter()
        .enumerate()
        .map(|(index, layer)| ParallelBatch {
            index,
            intents: layer
                .into_iter()
                .map(|position| graph.intents[position].clone())
                .collect(),
        })
        .collect::<Vec<_>>();

    let max_parallel_width = batches
        .iter()
        .map(|batch| batch.intents.len())
        .max()
        .unwrap_or(0);

    Ok(ResolutionReport {
        total_intents: graph.intents.len(),
        total_edges: graph.edges.len(),
        max_parallel_width,
        batches,
        edges: graph.edges,
    })
}

#[cfg(test)]
mod tests {
    use crate::error::DagResolutionError;
    use crate::model::Intent;

    use super::resolve_intents;

    fn intent(id: &str, reads: &[&str], writes: &[&str], after: &[&str]) -> Intent {
        Intent {
            id: id.to_string(),
            reads: reads.iter().map(|value| value.to_string()).collect(),
            writes: writes.iter().map(|value| value.to_string()).collect(),
            after: after.iter().map(|value| value.to_string()).collect(),
        }
    }

    #[test]
    fn groups_independent_intents_into_the_same_batch() {
        let report = resolve_intents(vec![
            intent("mint", &["cap"], &["supply"], &[]),
            intent("notify", &["status"], &[], &[]),
        ])
        .expect("independent intents should resolve");

        assert_eq!(report.batches.len(), 1);
        assert_eq!(report.max_parallel_width, 2);
        assert_eq!(report.batches[0].intents.len(), 2);
    }

    #[test]
    fn serializes_conflicting_writes_into_distinct_layers() {
        let report = resolve_intents(vec![
            intent("first", &[], &["vault"], &[]),
            intent("second", &["vault"], &["vault"], &[]),
        ])
        .expect("conflicting intents should still resolve");

        assert_eq!(report.batches.len(), 2);
        assert_eq!(report.batches[0].intents[0].id, "first");
        assert_eq!(report.batches[1].intents[0].id, "second");
    }

    #[test]
    fn rejects_explicit_cycles() {
        let err = resolve_intents(vec![
            intent("a", &[], &["vault"], &["b"]),
            intent("b", &[], &["vault"], &["a"]),
        ])
        .expect_err("cycle should be rejected");

        match err {
            DagResolutionError::CycleDetected(message) => {
                assert!(message.contains("a"));
                assert!(message.contains("b"));
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }
}
