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

use binary_heap_plus::*;
use priority_queue::PriorityQueue;
use std::collections::BinaryHeap;

use std::cmp::Ordering;

pub fn legal_block_cmp_y(a: &LegalBlock, b: &LegalBlock) -> Ordering {
    if a.y < b.y {
        return Ordering::Less;
    }
    if a.y > b.y {
        return Ordering::Greater;
    }
    Ordering::Equal
}

pub fn legal_block_cmp_x(a: &LegalBlock, b: &LegalBlock) -> Ordering {
    if a.x < b.x {
        return Ordering::Less;
    }
    if a.x > b.x {
        return Ordering::Greater;
    }
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
    pub upper_horizontal_weight: f32,
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

#[derive(Clone, Copy)]
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
        // println!("HARDMAX quick pack");
        return;
    }

    let root_opt = Option { index: 0, row: 0 };
    let root_opts = vec![root_opt];
    let root_node = Node {
        upper: 0.0,
        lower: 0.0,
        cost: 0.0,
        decision: true,
        block_index: 0,
    };
    let root_nodes = vec![root_node];
    let mut hcwt: HCwT<Context, Option, Node> = HCwT::new(root_opts, root_nodes);
    for i in 0..rowpair.blocks.len() {
        let mut dx = hcwt.new_decision();
        hcwt.add_option(&mut dx, Option { index: i, row: 0 });
        hcwt.add_option(&mut dx, Option { index: i, row: 1 });
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
fn generate(
    hcwt: &HCwT<Context, Option, Node>,
    context: &mut Context,
    level: usize,
    option_index: usize,
    parent_index: usize,
    nodes: &mut Vec<SelectionNode<Node>>,
) {
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
        new_node.cost += dx * dx + dy * dy; // Penalize horizontal shifts more?
                                            // new_node.cost += (dx*dx*dx).abs() + (dy*dy*dy).abs();
                                            // new_node.cost += (dx + dy).abs();
    } else {
        dx = new_node.upper - block.x;
        dy = context.rowpair.y1 - block.y;
        new_node.upper += block.w;
        new_node.decision = false;
        // Slightly less penalty for movement in the upper row
        new_node.cost += (dx * dx * context.rowpair.upper_horizontal_weight + dy * dy)
            * context.rowpair.upper_weight;
        // new_node.cost += ((dx*dx*dx).abs() + (dy*dy*dy).abs()) * context.rowpair.upper_weight;
        // new_node.cost += (dx + dy).abs();
    }

    // Check for difference in row lengths
    let delta = new_node.upper - new_node.lower;
    if delta.abs() <= context.rowpair.delta {
        nodes.push(SelectionNode {
            parent: parent_index,
            detail: new_node,
        });
    }
}

fn node_compare(a: &Node, b: &Node) -> Ordering {
    if a.lower < b.lower {
        return Ordering::Less;
    }
    if a.lower > b.lower {
        return Ordering::Greater;
    }
    if a.cost < b.cost {
        return Ordering::Less;
    }
    if a.cost > b.cost {
        return Ordering::Greater;
    }
    Ordering::Equal
}
fn filter(
    _hcwt: &HCwT<Context, Option, Node>,
    _context: &mut Context,
    level: usize,
    nodes: &mut Vec<SelectionNode<Node>>,
) -> Vec<SelectionNode<Node>> {
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
        if nodes[i].detail.lower != len {
            //  && cost > nodes[i].detail.cost {
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

fn trace(
    hcwt: &HCwT<Context, Option, Node>,
    level: usize,
    index: usize,
    upper: &mut Vec<usize>,
    lower: &mut Vec<usize>,
) {
    if level > 0 {
        trace(
            hcwt,
            level - 1,
            hcwt.levels[level].nodes[index].parent,
            upper,
            lower,
        );
        let nd = &hcwt.levels[level].nodes[index].detail;
        // println!("{} {}  cost {} decision: {} block {}", nd.lower, nd.upper, nd.cost, nd.decision, nd.block_index);
        if nd.decision {
            upper.push(nd.block_index);
        } else {
            lower.push(nd.block_index);
        }
    }
}

struct Blockage {
    pub start: f32,
    pub end: f32,
}

fn block_compare(a: &Blockage, b: &Blockage) -> Ordering {
    if a.start < b.start {
        return Ordering::Less;
    }
    if a.start > b.start {
        return Ordering::Greater;
    }
    Ordering::Equal
}
struct Row {
    pub target: f32,
    pub blockages: Vec<Blockage>,
}

struct Pool {
    pub blocks: Vec<LegalBlock>,
    pub start: f32, // X coordinates
    pub stop: f32,
    pub y: f32,      // Y location for the lower row
    pub target: f32, // How much per row
    pub filled: f32, // How much has been put into the pool
}

fn make_pools(row: &Row, end_row: f32) -> Vec<Pool> {
    let mut pools = Vec::new();
    let mut start = 0.0;
    for blockage in &row.blockages {
        if start < blockage.start {
            let target = blockage.start - start;
            pools.push(Pool {
                blocks: Vec::new(),
                start,
                stop: blockage.start,
                y: 0.0,
                target,
                filled: 0.0,
            });
        }
        start = blockage.end;
    }
    // And potentially one more pool after the last blockage
    if start < end_row {
        let target = end_row - start;
        pools.push(Pool {
            blocks: Vec::new(),
            start,
            stop: end_row,
            y: 0.0,
            target,
            filled: 0.0,
        });
    }

    // println!("Row----");
    #[cfg(feature = "ldbg")]
    for p in &pools {
        println!("Pool: {} to {}, target {}", p.start, p.stop, p.target);
    }

    pools
}

fn pool_distance(pool: &Pool, location: f32) -> f32 {
    if location < pool.start {
        return pool.start - location;
    }
    if location > pool.stop {
        return location - pool.stop;
    }
    0.0
}

fn add_to_pool(pools: &mut Vec<Pool>, block: &LegalBlock) {
    if pools.len() == 1 {
        pools[0].filled += block.w;
        pools[0].blocks.push(*block);
        return;
    }
    let mut best_pool = 0;
    let location = block.x + block.w / 2.0;
    let mut best_dist = pool_distance(&pools[0], location);
    for i in 1..pools.len() {
        let d = pool_distance(&pools[i], location);
        if d < best_dist {
            best_pool = i;
            best_dist = d;
        }
    }
    #[cfg(feature = "ldbg")]
    println!(
        "Add block {} with width {} to pool {} distance {} target {}",
        block.tag, block.w, best_pool, best_dist, pools[best_pool].target
    );
    pools[best_pool].filled += block.w;
    pools[best_pool].blocks.push(*block);
}

fn snap_macros(
    lp: &LegalProblem,
    target: f32,
    macros: &mut Vec<LegalBlock>,
    legal_positions: &mut Vec<LegalPosition>,
) {
    let top = lp.params.origin_y + lp.params.grid_y as f32 * lp.params.step_y;
    // let right = lp.params.grid_x as f32 * lp.params.step_x;
    let right = target;
    let deadband = 2.0 * lp.params.step_y;

    for block in macros {
        #[cfg(feature = "ldbg")]
        println!("Snap block {}", block.tag);
        let original_x = block.x;
        let original_y = block.y;
        let mut lblock = *block;
        lblock.x = (block.x / lp.params.step_x).round() * lp.params.step_x;
        lblock.y = (block.y / lp.params.step_y).round() * lp.params.step_y;

        // Shift in from the sides
        if lblock.x > right {
            lblock.x = right - lblock.w;
        }
        if lblock.x < lp.params.origin_x {
            lblock.x = lp.params.origin_x;
        }

        // Shift in from the top
        if lblock.y + lblock.h > top {
            lblock.y = top - lblock.h;
        }
        if lblock.y < lp.params.origin_y {
            lblock.y = lp.params.origin_y;
        }

        // Snap to the boundaries if close
        if right - (lblock.x + lblock.w) < deadband {
            #[cfg(feature = "ldbg")]
            println!("Snap block {} right because of deadband.", block.tag);
            lblock.x = right - lblock.w;
        }
        if lblock.x < deadband {
            #[cfg(feature = "ldbg")]
            println!("Snap block {} left because of deadband.", block.tag);
            lblock.x = 0.0;
        }

        if top - (lblock.y + lblock.h) < deadband {
            #[cfg(feature = "ldbg")]
            println!("Snap block {} up because of deadband.", block.tag);
            lblock.y = top - lblock.h;
        }
        if lblock.y < deadband {
            #[cfg(feature = "ldbg")]
            println!("Snap block {} down because of deadband.", block.tag);
            lblock.y = 0.0;
        }

        legal_positions.push(LegalPosition {
            block_tag: block.tag,
            x: lblock.x,
            y: lblock.y,
            h: block.h,
            w: block.w,
            original_x,
            original_y,
        });
        #[cfg(feature = "ldbg")]
        println!(
            "Shift {} from {} {} to {} {}",
            block.tag, block.x, block.y, lblock.x, lblock.y
        );
        block.x = lblock.x;
        block.y = lblock.y;
    }
}

fn legalize_mixed(lp: &LegalProblem) -> Vec<LegalPosition> {
    #[cfg(feature = "ldbg")]
    println!("SPECIAL MIXED HCWT");
    let mut legal_positions = Vec::new();
    // Find out exactly how much area we're using
    let mut area = 0.0;
    for block in &lp.blocks {
        area += block.h * block.w;
    }
    let target = (area / (lp.params.grid_y as f32 * lp.params.step_y)).round();
    #[cfg(feature = "ldbg")]
    println!("Target {} in each row", target);

    // Heap will contain only the standard cells
    let mut bhp = binary_heap_plus::BinaryHeap::new_by(|a: &LegalBlock, b: &LegalBlock| {
        legal_block_cmp_y(a, b).reverse()
    });

    let mut macros = Vec::new();

    // Find the macro blocks, put cells into the heap
    for block in &lp.blocks {
        if block.h > lp.params.step_y {
            macros.push(*block);
        } else {
            bhp.push(*block);
        }
    }
    snap_macros(lp, target, &mut macros, &mut legal_positions);

    let mut rows = Vec::new();
    // Now figure out the target amount in each row
    for row in 0..lp.params.grid_y {
        let mut newrow = Row {
            target,
            blockages: Vec::new(),
        };
        rows.push(newrow);
    }
    for mb in &macros {
        let lowrow = (mb.y / lp.params.step_y) as usize;
        let mut highrow = ((mb.y + mb.h) / lp.params.step_y) as usize;
        if highrow >= rows.len() {
            highrow = rows.len();
        }
        for row in lowrow..highrow {
            rows[row].target -= mb.w;
            rows[row].blockages.push(Blockage {
                start: mb.x,
                end: mb.x + mb.w,
            });
            #[cfg(feature = "ldbg")]
            println!("Block {} uses row {}, consumes {}", mb.tag, row, mb.w);
        }
    }
    for row in 0..lp.params.grid_y {
        rows[row].blockages.sort_by(|a, b| block_compare(&a, &b));
        // Fill the pools
        #[cfg(feature = "ldbg")]
        println!("Row {}", row);
        let mut pools = make_pools(&rows[row], target);
        let mut fill = 0.0;
        for p in &pools {
            fill += p.target;
        }
        fill = fill * 2.0;
        let mut taken = 0.0;

        // Now fill up the pools
        while !bhp.is_empty() && taken < fill {
            let block = bhp.pop().unwrap();
            taken += block.w;
            add_to_pool(&mut pools, &block);
        }

        // Now run HCwT for each pool
        for p in &pools {
            #[cfg(feature = "ldbg")]
            println!("POOL {} to {}", p.start, p.stop);
            let mut rowpair = HcwtRowPair {
                blocks: p.blocks.clone(),
                x: p.start,
                y0: lp.params.origin_y + row as f32 * lp.params.step_y,
                y1: lp.params.origin_y + (row + 2) as f32 * lp.params.step_y,
                length: p.target,
                hard_max: p.target + 10.0,
                delta: 50.0,
                upper_weight: 0.8,
                upper_horizontal_weight: 0.1,
                upper: Vec::new(),
                lower: Vec::new(),
            };
            rowpair.blocks.sort_by(|a, b| legal_block_cmp_x(a, b));
            pack_row_hcwt(&mut rowpair);
            let mut x = p.start;
            for block in rowpair.lower {
                // println!("  Block {} to {}", block.tag, x);
                legal_positions.push(LegalPosition {
                    block_tag: block.tag,
                    x,
                    y: rowpair.y0,
                    h: block.h,
                    w: block.w,
                    original_x: block.x,
                    original_y: block.y,
                });
                x += block.w;
                // row_taken += block.w;
                // total_taken += block.w;
            }
            for block in rowpair.upper {
                bhp.push(block);
            }
        }
    }

    if legal_positions.len() != lp.blocks.len() {
        println!(
            "Incorrect number of blocks legalized {} vs {}",
            legal_positions.len(),
            lp.blocks.len()
        );
        println!("Heap has {} entries", bhp.len());
    }

    legal_positions
}
pub fn legalize(lp: &LegalProblem) -> Vec<LegalPosition> {
    #[cfg(feature = "ldbg")]
    println!("HCWT placement legalizer");
    // See if we have any macro blocks -- if so, we need to use the mixed legalizer
    for block in &lp.blocks {
        if block.h > lp.params.step_y {
            return legalize_mixed(lp);
        }
    }

    let mut legal_positions = Vec::new();

    // Total length of all cells
    let mut total_length = 0.0;
    // As cells are legalied, we take the length away.
    // Use this to determine what the best lower-row target length is
    let mut total_taken = 0.0;

    let mut bhp = binary_heap_plus::BinaryHeap::new_by(|a: &LegalBlock, b: &LegalBlock| {
        legal_block_cmp_y(a, b).reverse()
    });
    for block in &lp.blocks {
        total_length += block.w;
        bhp.push(*block);
    }

    let target = total_length / lp.params.grid_y as f32;
    let avg_cell = total_length / lp.blocks.len() as f32;

    #[cfg(feature = "ldbg")]
    println!(
        "Total length: {}  row length: {}  average cell width {}",
        total_length,
        total_length / lp.params.grid_y as f32,
        total_length / lp.blocks.len() as f32
    );
    println!("{} rows", lp.params.grid_y);

    let mut row_num = 0;
    while !bhp.is_empty() {
        let row_target = (total_length - total_taken) / (lp.params.grid_y - row_num) as f32;
        let mut rowpair = HcwtRowPair {
            blocks: Vec::new(),
            x: lp.params.origin_x,
            y0: lp.params.origin_y + row_num as f32 * lp.params.step_y,
            y1: lp.params.origin_y + (row_num + 2) as f32 * lp.params.step_y,
            length: row_target,
            hard_max: row_target + avg_cell * 10.0,
            delta: avg_cell * 15.0,
            upper_weight: 0.8,
            upper_horizontal_weight: 0.1,
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

        // If we're near the last row, put the horizontal displacement up, because we're not
        // going to be able to recover
        if row_num >= lp.params.grid_y - 2 {
            rowpair.upper_horizontal_weight = 0.6;
        }
        pack_row_hcwt(&mut rowpair);

        // Lower row gets packed, upper row goes back into the hopper
        let mut x = lp.params.origin_x;
        let mut row_taken = 0.0;
        for block in rowpair.lower {
            legal_positions.push(LegalPosition {
                block_tag: block.tag,
                x,
                y: rowpair.y0,
                h: block.h,
                w: block.w,
                original_x: block.x,
                original_y: block.y,
            });
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
