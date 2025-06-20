use bookshelf_r::bookshelf::BookshelfCircuit;
use super::{LegalBlock, LegalParams, LegalPosition, LegalProblem};

pub fn legalize(lp: &LegalProblem) -> Vec<LegalPosition> {
    // println!("Tetris placement legalizer"); // (optimized with directional cost)

    let mut blocks = lp.blocks.clone();
    for i in 0..blocks.len() {
        blocks[i].tag = i;
    }
    let params = &lp.params;

    // Sort blocks by their preferred X position
    // Sorting: prioritize blocks on the left
    blocks.sort_by(|a, b| a.x.partial_cmp(&b.x).expect("Could not compare"));

    //Initialize left edge
    let mut legal_positions = Vec::new();
    let mut left_edges = vec![params.origin_x; params.grid_y];
    let mut row_usage = vec![0usize; params.grid_y]; // Track usage of each line

    // Direction-sensitive cost factor
    //const ALPHA_RIGHT: f32 = 2.0; // Penalty factor for moving to the right (higher)
    //const ALPHA_LEFT: f32 = 0.5;  // The reward factor for moving left (lower)
    const BETA: f32 = 0.5;         // Row congestion penalty coefficient

    //Go through each block and find the best place to put it
    for block in &blocks {
        // Modified: Use ceil() to calculate required rows and ensure minimum 1 row
        let block_rows = (block.h / params.step_y).ceil() as usize;
        let block_rows = block_rows.max(1); // Ensure at least 1 row
        
        // Modified: Dynamic search range calculation with floor() for safety
        let best_row = ((block.y - params.origin_y) / params.step_y).floor() as usize;
        let search_radius = (5 * params.grid_y / 100).max(5); // At least 5 rows or 5% of total
        let low_row = best_row.saturating_sub(search_radius);
        let high_row = (best_row + search_radius).min(params.grid_y.saturating_sub(block_rows));

        let mut best_row = best_row;
        let mut best_cost = f32::MAX;
        let mut best_x = params.origin_x;

        for row in low_row..=high_row {
            // Modified: Safer multi-row left edge calculation
            let row_range = row..(row + block_rows);
            let left = row_range.clone()
                .map(|r| left_edges.get(r).unwrap_or(&params.origin_x))
                .fold(params.origin_x, |a, &b| a.max(b));

            // Original congestion calculation with dynamic beta
            let dynamic_beta = BETA * (1.0 + row_usage[row] as f32 / 10.0);

            // Original direction-sensitive cost calculation
            let delta_x = left - block.x;
            let alpha = if delta_x > 0.0 { 
                params.alpha_right  // Move right penalty
            } else { 
                params.alpha_left  // Move Left Reward
            };

            // Modified: Improved Y-displacement calculation considering height
            let placed_y = params.origin_y + row as f32 * params.step_y;
            let delta_y = (block.y - placed_y).abs() + 
                          (block.h - (block_rows as f32 * params.step_y)).abs() * 0.1;

            let row_crowding = dynamic_beta * (row_usage[row] as f32);
            let cost = delta_y + alpha * delta_x.abs() + row_crowding;

            if cost < best_cost {
                best_row = row;
                best_cost = cost;
                best_x = left;
            }
        }

        // Modified: Enhanced boundary checking with better error message
        assert!(
            best_row + block_rows <= params.grid_y,
            "Vertical placement out of bounds for block {} (required rows: {}, available: {})",
            block.tag, block_rows, params.grid_y - best_row
        );

        // Record legalization location
        legal_positions.push(LegalPosition {
            block: block.tag,
            x: best_x,
            y: params.origin_y + best_row as f32 * params.step_y,
        });

        // Update left margin and row usage count
        for r in best_row..best_row + block_rows {
            left_edges[r] = best_x + block.w;
            row_usage[r] += 1;
        }
    }

    legal_positions
}

pub fn legalize_floorplan(lp: &LegalProblem) -> Vec<LegalPosition> {
    // println!("Floorplan legalizer with x-compaction (Tetris-style left-packing)");

    let mut blocks = lp.blocks.clone();

    // Replace the block ID with the index into the original list.  This way, we can
    // avoid searching for a matching block tag
    for i in 0..blocks.len() {
        blocks[i].tag = i;
    }

    let params = &lp.params;

    //Identify all unique y positions (y1 & y2 for each block)
    let mut y_points = Vec::new();
    for block in &blocks {
        y_points.push(block.y);
        y_points.push(block.y + block.h);
    }

    //Sort and deduplicate the y positions
    y_points.sort_by(|a, b| a.partial_cmp(b).unwrap());
    y_points.dedup();

    //Map each vertical span (y1..y2) to a mutable left edge X position
    let mut y_segments = vec![params.origin_x; y_points.len() - 1];

    //find index of a y value in y_points
    let find_y_index = |y: f32| -> usize {
        y_points.binary_search_by(|probe| probe.partial_cmp(&y).unwrap())
            .unwrap_or_else(|i| i.saturating_sub(1))
    };

    //Sort blocks by x (left to right)
    blocks.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap());

    let mut legal_positions = Vec::new();

    for block in &blocks {
        let y_start = find_y_index(block.y);
        let y_end = find_y_index(block.y + block.h);

        //Determine the left-most X that this block can be placed at
        let mut max_x = params.origin_x;
        for y_idx in y_start..y_end {
            if y_segments[y_idx] > max_x {
                max_x = y_segments[y_idx];
            }
        }

        // lace the block at max x
        legal_positions.push(LegalPosition {
            block: block.tag,
            x: max_x,
            y: block.y, //y unchange
        });

        //Update all y_segments that this block covers
        for y_idx in y_start..y_end {
            y_segments[y_idx] = max_x + block.w;
        }
    }

    legal_positions
}
