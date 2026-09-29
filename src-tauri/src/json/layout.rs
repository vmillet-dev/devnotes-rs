//! Left to right: one column per level, each as wide as its widest node, and a child level
//! with the row that opens it whenever the nodes above leave the room.

use super::model::JsonNode;

/// Places the nodes, parents listed before their children, and answers the extent drawn.
pub(crate) fn place(nodes: &mut [JsonNode], column_gap: u32, node_gap: u32) -> (u32, u32) {
    if nodes.is_empty() {
        return (0, 0);
    }

    let mut depth = vec![0_usize; nodes.len()];
    let mut children: Vec<Vec<usize>> = vec![Vec::new(); nodes.len()];
    for index in 1..nodes.len() {
        let parent = nodes[index].parent.map_or(0, |parent| parent as usize);
        depth[index] = depth[parent] + 1;
        children[parent].push(index);
    }

    let levels = depth.iter().max().map_or(1, |deepest| deepest + 1);
    let mut column_width = vec![0; levels];
    for (index, node) in nodes.iter().enumerate() {
        column_width[depth[index]] = column_width[depth[index]].max(node.width);
    }
    let mut column_x = vec![0; levels];
    for level in 1..levels {
        column_x[level] = column_x[level - 1] + column_width[level - 1] + column_gap;
    }
    for (index, node) in nodes.iter_mut().enumerate() {
        node.x = column_x[depth[index]];
    }

    let bottom = subtree(nodes, &children, 0, 0, node_gap);
    let width = column_x[levels - 1] + column_width[levels - 1];
    (width, bottom)
}

/// Where the subtree under `index` ends, placed from `top` down.
fn subtree(
    nodes: &mut [JsonNode],
    children: &[Vec<usize>],
    index: usize,
    top: u32,
    gap: u32,
) -> u32 {
    nodes[index].y = top;
    let mut bottom = top + nodes[index].height;
    let mut next = top;

    for &child in &children[index] {
        let id = nodes[child].id;
        let row = nodes[index]
            .rows
            .iter()
            .position(|row| row.child == Some(id))
            .map_or(0, |row| u32::try_from(row).unwrap_or(u32::MAX));
        let start = next.max(top + 1 + row);
        let end = subtree(nodes, children, child, start, gap);
        bottom = bottom.max(end);
        next = end + gap;
    }

    bottom
}
