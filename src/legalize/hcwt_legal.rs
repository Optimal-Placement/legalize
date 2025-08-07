use bookshelf_r::bookshelf::BookshelfCircuit;
use hcwt_r;

use super::{LegalBlock, LegalParams, LegalPosition, LegalProblem};

/*pub fn legalize(lp: &LegalProblem) -> Vec<LegalPosition> {
    println!("HCWT placement legalizer");
    // HCwT legalizer
    // Use dynamic programming, to stack up a row at a time.  Put all of the blocks into
    // a heap, ordered by the Y axis.  Then, select 2X of the row length as candidates for
    // legalization
    //
    // Build an HCwT problem, where each cell is a decision (top row or bottom row).
    // Insert as needed, keep a running total on each node, and then find the minimum
    // displacement packing.
    //
    // Then -- the lower row "stays", the upper row goes back into the heap for another
    // attempt.
    //
    // Return a vector of where the blocks get moved to
    Vec::new()
}*/

use priority_queue::PriorityQueue;
use std::collections::BinaryHeap;
use binary_heap_plus::*;

use std::cmp::Ordering;

pub fn legal_block_cmp_y(a: &LegalBlock, b: &LegalBlock) -> Ordering {
    if a.y < b.y {return Ordering::Less;}
    if a.y > b.y {return Ordering::Greater;}
    Ordering::Equal
}

pub fn legal_block_cmp_x(a: &LegalBlock, b: &LegalBlock) -> Ordering {
    if a.x < b.x {return Ordering::Less;}
    if a.x > b.x {return Ordering::Greater;}
    Ordering::Equal
}

#[derive(Clone)]
struct HcwtRowPair {
    pub blocks: Vec<LegalBlock>,
    pub x: f32,
    pub y0: f32,
    pub y1: f32,
    pub length: f32,
    pub hard_max: f32,
    pub delta: f32,
    pub upper_weight: f32,
    pub upper: Vec<LegalBlock>,
    pub lower: Vec<LegalBlock>,
}

fn pack_row(rowpair: &mut HcwtRowPair) {
    // Right now, just put the first ones into the lower, then the rest in the upper
    let mut len0 = 0.0;
    let mut len1 = 0.0;
    for block in &rowpair.blocks {
        if len0 < rowpair.length {
            len0 += block.w;
            rowpair.lower.push(*block);
        } else {
            rowpair.upper.push(*block);
            len1 += block.w;
        }
    }
    println!("Row pair: {} {}", len0, len1);
}

#[derive(Clone,Copy)]
struct Node {
    upper: f32,
    lower: f32,
    cost: f32,
    decision: bool,
    block_index: usize,
}

struct Context {
    rowpair: HcwtRowPair,
}

struct Option {
    pub index: usize,
    pub row: usize,
}
use hcwt_r::HCwT;
use hcwt_r::SelectionNode;

fn pack_row_hcwt(rowpair: &mut HcwtRowPair) {
    // Check to see if everything can be packed in the
    // lower row
    let mut len = 0.0;
    for b in &rowpair.blocks {
        len += b.w;
    }
    if len < rowpair.hard_max {
        for b in &rowpair.blocks {
            rowpair.lower.push(*b);
        }
        println!("HARDMAX quick pack");
        return;
    }

    let root_opt = Option {
        index: 0, 
        row: 0,   
    };
    let root_opts = vec![root_opt];
    let root_node = Node {
        upper: 0.0,
        lower: 0.0,
        cost: 0.0,
        decision: true,
        block_index: 0,
    };
    let root_nodes = vec![root_node];
    let mut hcwt: HCwT<Context,Option,Node> = HCwT::new(root_opts, root_nodes);
    for i in 0..rowpair.blocks.len() {
        let mut dx = hcwt.new_decision();
        hcwt.add_option(&mut dx, Option{index: i, row: 0});
        hcwt.add_option(&mut dx, Option{index: i, row: 1});
        hcwt.add_decision(dx);
    }
    let mut share_context = Context {
        rowpair: rowpair.clone(),
    };

    hcwt.solve(&mut share_context, &generate, &filter);
    let last = &hcwt.levels.last().unwrap().nodes;
    let level = hcwt.levels.len() - 1;
    let mut best = 0;
    let mut mismatch = rowpair.length;

    // Find the row length that is closest to the desired length
    for i in 0..last.len() {
        let node = &last[i];
        // println!("Final level solution {} {}  cost {}", node.detail.lower, node.detail.upper, node.detail.cost);
        let delta = (rowpair.length - node.detail.lower).abs();
        if delta < mismatch {
            best = i;
            mismatch = delta;
        }
    }
    let mut upper = Vec::new();
    let mut lower = Vec::new();
    trace(&hcwt, level, best, &mut lower, &mut upper);

    for low in lower {
        rowpair.lower.push(rowpair.blocks[low]);
    }
    for high in upper {
        rowpair.upper.push(rowpair.blocks[high]);
        // rowpair.lower.push(rowpair.blocks[high]);
    }

}
fn generate(hcwt: &HCwT<Context,Option,Node>, context: &mut Context, level: usize, option_index: usize, parent_index: usize, nodes: &mut Vec<SelectionNode<Node>>) {
    let parent_node = &hcwt.levels[level - 1].nodes[parent_index];
    let detail = &hcwt.levels[level].options[option_index];

    let mut new_node = parent_node.detail;
    new_node.block_index = detail.index;    
    let block = &context.rowpair.blocks[detail.index];
    let dx;
    let dy;
    if detail.row == 0 {
        dx = new_node.lower - block.x;
        dy = context.rowpair.y0 - block.y;
        new_node.lower += block.w;
        new_node.decision = true;
        new_node.cost += dx*dx*10.0 + dy*dy; // Penalize horizontal shifts more?
        // new_node.cost += (dx*dx*dx).abs() + (dy*dy*dy).abs();
        // new_node.cost += (dx + dy).abs();
    } else {
        dx = new_node.upper - block.x;
        dy = context.rowpair.y1 - block.y;
        new_node.upper += block.w;
        new_node.decision = false;
        // Slightly less penalty for movement in the upper row
        new_node.cost += (dx*dx + dy*dy) * context.rowpair.upper_weight;
        // new_node.cost += ((dx*dx*dx).abs() + (dy*dy*dy).abs()) * context.rowpair.upper_weight;
        // new_node.cost += (dx + dy).abs();        
    }

    // Check for difference in row lengths
    let delta = new_node.upper - new_node.lower;
    if delta.abs() <= context.rowpair.delta {
        
        nodes.push(SelectionNode{parent: parent_index, detail: new_node});
    }
}

fn node_compare(a: &Node, b: &Node) -> Ordering {
    if a.lower < b.lower {return Ordering::Less;}
    if a.lower > b.lower {return Ordering::Greater;}
    if a.cost < b.cost {return Ordering::Less;}
    if a.cost > b.cost {return Ordering::Greater;}
    Ordering::Equal
}
fn filter(_hcwt: &HCwT<Context,Option,Node>, _context: &mut Context, level: usize, nodes: &mut Vec<SelectionNode<Node>>) -> Vec<SelectionNode<Node>>
{
    let mut result = Vec::new();
    // println!("Filter level {level}");
    if nodes.len() == 0 {
        println!("NO NODES!");
        return result;
    }
    nodes.sort_by(|a, b| node_compare(&a.detail, &b.detail));
    let mut len = nodes[0].detail.lower;
    let mut cost = nodes[0].detail.cost;
    result.push(nodes[0]);
    // println!("Node 0 length {}:{} cost {}", len, nodes[0].detail.upper, cost);
    for i in 1..nodes.len() {
        // The lowest cost result for any given length is of interest
        if nodes[i].detail.lower != len { //  && cost > nodes[i].detail.cost {
            result.push(nodes[i]);
            len = nodes[i].detail.lower;
            cost = nodes[i].detail.cost;
            // println!("Save node {} length {}:{} cost {}", i, nodes[i].detail.lower, nodes[i].detail.upper, nodes[i].detail.cost);
        } else {
            // println!("   x node {} length {}:{} cost {}", i, nodes[i].detail.lower, nodes[i].detail.upper, nodes[i].detail.cost);
        }
    }

    result
}

fn trace(hcwt: &HCwT<Context,Option,Node>, level: usize, index: usize, upper: &mut Vec<usize>, lower: &mut Vec<usize>) {
    if level > 0 {
        trace(hcwt, level - 1, hcwt.levels[level].nodes[index].parent, upper, lower);
        let nd = &hcwt.levels[level].nodes[index].detail;
        // println!("{} {}  cost {} decision: {} block {}", nd.lower, nd.upper, nd.cost, nd.decision, nd.block_index);
        if nd.decision {
            upper.push(nd.block_index);
        } else {
            lower.push(nd.block_index);
        }
    }

}

pub fn legalize(lp: &LegalProblem) -> Vec<LegalPosition> {
    println!("HCWT placement legalizer");
    let mut legal_positions = Vec::new();

    // Total length of all cells
    let mut total_length = 0.0;
    // As cells are legalied, we take the length away.
    // Use this to determine what the best lower-row target length is
    let mut total_taken = 0.0;
    
    let mut bhp = binary_heap_plus::BinaryHeap::new_by(|a: &LegalBlock, b: &LegalBlock| legal_block_cmp_y(a, b).reverse());
    for block in &lp.blocks {
        total_length += block.w;
        bhp.push(*block);
    }

    let target = total_length / lp.params.grid_y as f32;
    let avg_cell = total_length / lp.blocks.len() as f32;
    
    println!("Total length: {}  row length: {}  average cell width {}", total_length, total_length / lp.params.grid_y as f32, total_length/lp.blocks.len() as f32);
    println!("{} rows", lp.params.grid_y);

    let mut row_num = 0;    
    while !bhp.is_empty() {
        let row_target = (total_length - total_taken)/(lp.params.grid_y - row_num) as f32;
        let mut rowpair = HcwtRowPair {
            blocks: Vec::new(),
            x: lp.params.origin_x,
            y0: lp.params.origin_y + row_num as f32 * lp.params.step_y,
            y1: lp.params.origin_y + (row_num + 3) as f32 * lp.params.step_y,
            length: row_target,
            hard_max: row_target + avg_cell * 10.0,
            delta: avg_cell * 15.0,
            upper_weight: 1.0,
            upper: Vec::new(),
            lower: Vec::new(),
        };
        let mut taken = 0.0;
        while taken < (target * 2.0 + avg_cell * 5.0) && !bhp.is_empty() {
            let block = bhp.pop().unwrap();
            taken += block.w;
            rowpair.blocks.push(block);
        }
        rowpair.blocks.sort_by(|a, b| legal_block_cmp_x(a, b));

        // println!("Row {}", row_num);
        // pack_row(&mut rowpair);
        pack_row_hcwt(&mut rowpair);



        // Lower row gets packed, upper row goes back into the hopper
        let mut x = lp.params.origin_x;
        let mut row_taken = 0.0;
        for block in rowpair.lower {
            legal_positions.push(LegalPosition {block_tag: block.tag, x, y: rowpair.y0, h: block.h, w: block.w, original_x: block.x, original_y: block.y});
            x += block.w;
            row_taken += block.w;
            total_taken += block.w;
        }
        // println!("Row {} take {}", row_num, row_taken);
        for block in rowpair.upper {
            bhp.push(block);
        }

        row_num = row_num + 1;
    }
    
    legal_positions
}