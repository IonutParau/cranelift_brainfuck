use crate::parser::Node;

pub fn optimize(nodes: &[Node]) -> Vec<Node> {
    let mut optimized_nodes = vec![];

    for node in nodes {
        optimized_nodes.push(node.clone());
    }

    let mut l = 0;

    while optimized_nodes.len() != l {
        let mut next_nodes: Vec<Node> = vec![];

        for node in optimized_nodes.iter() {
            let old = next_nodes.pop();
            if let Some(old) = old {
                if let Node::Add(node_n) = node {
                    if let Node::Add(old_n) = old {
                        next_nodes.push(Node::Add(node_n.wrapping_add(old_n)));
                        continue;
                    }
                }
                if let Node::ShiftLeft(node_left) = node {
                    if let Node::ShiftLeft(old_left) = old {
                        next_nodes.push(Node::ShiftLeft(node_left + old_left));
                        continue;
                    }
                }
                if let Node::ShiftRight(node_right) = node {
                    if let Node::ShiftRight(old_right) = old {
                        next_nodes.push(Node::ShiftRight(node_right + old_right));
                        continue;
                    }
                }
                if let Node::ShiftLeft(node_left) = node {
                    if let Node::ShiftRight(old_right) = old {
                        if *node_left == old_right {
                            continue;
                        }
                        if old_right > *node_left {
                            next_nodes.push(Node::ShiftRight(old_right - node_left));
                        }
                        if *node_left > old_right {
                            next_nodes.push(Node::ShiftLeft(node_left - old_right));
                        }
                        continue;
                    }
                }
                if let Node::ShiftRight(node_right) = node {
                    if let Node::ShiftLeft(old_left) = old {
                        if *node_right == old_left {
                            continue;
                        }
                        if old_left > *node_right {
                            next_nodes.push(Node::ShiftLeft(old_left - node_right));
                        }
                        if *node_right > old_left {
                            next_nodes.push(Node::ShiftRight(node_right - old_left));
                        }
                        continue;
                    }
                }

                next_nodes.push(old);
            }
            next_nodes.push(*node);
        }

        l = optimized_nodes.len();
        optimized_nodes = next_nodes;
    }

    optimized_nodes
}