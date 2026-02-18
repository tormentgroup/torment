pub mod commands;

use std::collections::{BTreeMap, BTreeSet, HashSet, VecDeque};

use matrix_sdk::ruma::{MilliSecondsSinceUnixEpoch, OwnedRoomId, RoomId};

/// Acyclic directed graph of spaces built from `m.space.child` edges.
///
/// Edges are sorted oldest-first so the original parent→child relationship is
/// established before any back-links. Any edge that would form a cycle is
/// dropped; the older edge always wins.
pub(crate) struct SpaceGraph {
    dag: BTreeMap<OwnedRoomId, BTreeSet<OwnedRoomId>>,
    nested: HashSet<OwnedRoomId>,
}

impl SpaceGraph {
    pub fn build(
        edges: impl IntoIterator<
            Item = (OwnedRoomId, OwnedRoomId, Option<MilliSecondsSinceUnixEpoch>),
        >,
    ) -> Self {
        let mut sorted_edges: Vec<_> = edges.into_iter().collect();
        sorted_edges.sort_by(|a, b| match (&a.2, &b.2) {
            (Some(ta), Some(tb)) => ta.cmp(tb),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        });

        let mut dag: BTreeMap<OwnedRoomId, BTreeSet<OwnedRoomId>> = BTreeMap::new();
        let mut nested = HashSet::new();

        for (parent, child, _) in sorted_edges {
            if is_reachable(&dag, &child, &parent) {
                continue;
            }
            dag.entry(parent).or_default().insert(child.clone());
            nested.insert(child);
        }

        Self { dag, nested }
    }

    pub fn nested_ids(&self) -> &HashSet<OwnedRoomId> {
        &self.nested
    }

    /// Returns true if `child` is a valid (non-cyclic) child of `parent` in the DAG.
    pub fn is_valid_child(&self, parent: &RoomId, child: &RoomId) -> bool {
        self.dag
            .get(parent)
            .map_or(false, |children| children.contains(child))
    }
}

fn is_reachable(
    graph: &BTreeMap<OwnedRoomId, BTreeSet<OwnedRoomId>>,
    from: &OwnedRoomId,
    to: &OwnedRoomId,
) -> bool {
    let mut queue = VecDeque::from([from.clone()]);
    let mut visited = HashSet::new();
    while let Some(node) = queue.pop_front() {
        if &node == to {
            return true;
        }
        if !visited.insert(node.clone()) {
            continue;
        }
        if let Some(children) = graph.get(&node) {
            queue.extend(children.iter().cloned());
        }
    }
    false
}
