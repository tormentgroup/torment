use std::collections::{BTreeMap, BTreeSet, HashSet, VecDeque};

use matrix_sdk::ruma::{MilliSecondsSinceUnixEpoch, OwnedRoomId, RoomId, events::space::child::SpaceChildEventContent};

/// Acyclic directed graph of spaces built from `m.space.child` edges.
///
/// Edges are sorted oldest-first so the original parent→child relationship is
/// established before any back-links. Any edge that would form a cycle is
/// dropped; the older edge always wins.
pub(crate) struct SpaceGraph {
    dag: BTreeMap<OwnedRoomId, BTreeSet<OwnedRoomId>>,
    nested: HashSet<OwnedRoomId>,
}

pub(crate) struct SpaceGraphEdge {
    from: OwnedRoomId,
    to: OwnedRoomId,
    timestamp: Option<MilliSecondsSinceUnixEpoch>,
}

impl SpaceGraph {
    pub fn build(
        edges: impl IntoIterator<
            Item = SpaceGraphEdge,
        >,
    ) -> Self {
        let mut sorted_edges: Vec<_> = edges.into_iter().collect();
        sorted_edges.sort_by(|a, b| match (&a.timestamp, &b.timestamp) {
            (Some(ta), Some(tb)) => ta.cmp(tb),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        });

        let mut dag: BTreeMap<OwnedRoomId, BTreeSet<OwnedRoomId>> = BTreeMap::new();
        let mut nested = HashSet::new();

        for edge in sorted_edges {
            if is_reachable(&dag, &edge.to, &edge.from) {
                continue;
            }
            dag.entry(edge.from).or_default().insert(edge.to.clone());
            nested.insert(edge.to);
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

/// Collect all inter-space `m.space.child` edges from joined spaces.
pub async fn collect_space_edges(
    client: &matrix_sdk::Client,
) -> (Vec<matrix_sdk::Room>, Vec<SpaceGraphEdge>) {
    let all_spaces: Vec<_> = client
        .joined_rooms()
        .into_iter()
        .filter(|r| r.room_type().as_ref().map(|t| t.as_str()) == Some("m.space"))
        .collect();

    let joined_space_ids: std::collections::HashSet<OwnedRoomId> =
        all_spaces.iter().map(|s| s.room_id().to_owned()).collect();

    let mut edges = Vec::new();
    for space in &all_spaces {
        let Ok(child_events) = space.get_state_events_static::<SpaceChildEventContent>().await
        else {
            continue;
        };
        for raw_event in child_events {
            let Ok(event) = raw_event.deserialize() else {
                continue;
            };
            let child_id = event.state_key().clone();
            if !joined_space_ids.contains(&*child_id) {
                continue;
            }
            edges.push(SpaceGraphEdge {
                from: space.room_id().to_owned(),
                to: child_id,
                timestamp: event.origin_server_ts(),
            });
        }
    }

    (all_spaces, edges)
}
