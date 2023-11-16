#[derive(Debug, Clone, Copy)]
pub enum Token {
    Plus,
    Sub,
    Print,
    Read,
    ShiftLeft,
    ShiftRight,
    BeginLoop,
    EndLoop,
    Ignored,
}

// Name may be a bit confusing: This is a representation of an action,
// with basic optimizations or extra information.
#[derive(Debug, Clone, Copy)]
pub enum Node {
    Add(u8),
    Print,
    Read,
    ShiftLeft(u64),
    ShiftRight(u64),

    // The number is the ID.
    BeginLoop(u32),
    EndLoop(u32),
}

pub fn tokens(s: &str) -> Vec<Token> {
    let mut t = vec![];

    for c in s.chars() {
        use Token::*;

        t.push(match c {
            '+' => Plus,
            '-' => Sub,
            '.' => Print,
            ',' => Read,
            '<' => ShiftLeft,
            '>' => ShiftRight,
            '[' => BeginLoop,
            ']' => EndLoop,
            _ => Ignored,
        });
    }

    t
}

pub fn parse(tokens: &[Token]) -> Vec<Node> {
    let mut v = vec![];

    let mut loop_id = 0;
    let mut loop_stack = vec![];

    let l = tokens.len();

    for i in 0..l {
        let token = tokens[i].clone();

        match token {
            Token::Ignored => {}
            Token::Print => v.push(Node::Print),
            Token::Read => v.push(Node::Read),
            Token::ShiftLeft => v.push(Node::ShiftLeft(1)),
            Token::ShiftRight => v.push(Node::ShiftRight(1)),
            Token::BeginLoop => {
                loop_stack.push(loop_id);
                v.push(Node::BeginLoop(loop_id));
                loop_id += 1;
            }
            Token::EndLoop => {
                v.push(Node::EndLoop(
                    loop_stack.pop().expect("End of loop with no beginning"),
                ));
            }
            Token::Plus => v.push(Node::Add(1)),
            Token::Sub => v.push(Node::Add(255)),
        }
    }

    let mut optimized_nodes = v.clone();

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
