pub mod commands;

use std::collections::{BTreeMap, BTreeSet, HashSet, VecDeque};

use matrix_sdk::ruma::{MilliSecondsSinceUnixEpoch, OwnedRoomId};

/// Returns the set of space IDs that are nested (have a parent in the acyclic hierarchy).
///
/// Collects inter-space `m.space.child` edges sorted oldest-first so the original
/// parent→child relationship is established before any back-links. Any edge that
/// would form a cycle is skipped; the older edge always wins.
pub(crate) fn nested_space_ids(
    edges: impl IntoIterator<Item = (OwnedRoomId, OwnedRoomId, Option<MilliSecondsSinceUnixEpoch>)>,
    all_space_ids: impl IntoIterator<Item = OwnedRoomId>,
) -> HashSet<OwnedRoomId> {
    let all_ids: Vec<OwnedRoomId> = all_space_ids.into_iter().collect();

    let mut sorted_edges: Vec<(OwnedRoomId, OwnedRoomId, Option<MilliSecondsSinceUnixEpoch>)> =
        edges.into_iter().collect();
    sorted_edges.sort_by(|a, b| match (&a.2, &b.2) {
        (Some(ta), Some(tb)) => ta.cmp(tb),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    });

    let mut graph_children: BTreeMap<OwnedRoomId, BTreeSet<OwnedRoomId>> =
        all_ids.iter().map(|id| (id.clone(), Default::default())).collect();
    let mut graph_parents: BTreeMap<OwnedRoomId, BTreeSet<OwnedRoomId>> =
        all_ids.iter().map(|id| (id.clone(), Default::default())).collect();

    for (parent, child, _) in sorted_edges {
        if is_reachable(&graph_children, &child, &parent) {
            continue;
        }
        graph_children.entry(parent.clone()).or_default().insert(child.clone());
        graph_parents.entry(child).or_default().insert(parent);
    }

    graph_parents
        .into_iter()
        .filter(|(_, parents)| !parents.is_empty())
        .map(|(id, _)| id)
        .collect()
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
