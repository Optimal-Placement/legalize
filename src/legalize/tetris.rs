use bookshelf_r::bookshelf::BookshelfCircuit;

use super::{LegalBlock, LegalParams, LegalPosition, LegalProblem};

/*pub fn legalize(lp: &LegalProblem) -> Vec<LegalPosition> {
    println!("Tetris placement legalizer");
    Vec::new()
}
*/


pub fn legalize(lp: &LegalProblem) -> Vec<LegalPosition> {
    println!("Tetris placement legalizer"); // (optimized with directional cost)

    let mut blocks = lp.blocks.clone();
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
    const BETA: f32 = 0.3;         // Row congestion penalty coefficient


    //Go through each block and find the best place to put it
    for block in &blocks {
        // cal row
        let block_rows = (block.h / params.step_y).round() as usize;
        let best_row = ((block.y - params.origin_y) / params.step_y).round() as usize;
        
        // Search range: 5 lines up and down (adjustable as needed)
        let low_row = best_row.saturating_sub(5);
        let high_row = (best_row + 5).min(params.grid_y.saturating_sub(block_rows));


        //Find the location of min displacement near the optimal row
        let mut best_row = best_row;
        let mut best_cost = f32::MAX;
        let mut best_x = params.origin_x;

        for row in low_row..=high_row {
            // Calculates the available left margin of the current line
            let mut left = left_edges[row];
            for r in row..row + block_rows {
                left = left.max(left_edges[r]);
            }
        // Dynamically adjust BETA based on row usage
        let dynamic_beta = BETA * (1.0 + row_usage[row] as f32 / 10.0);


         // Calculating direction-sensitive costs
         let delta_x = left - block.x;
         let alpha = if delta_x > 0.0 { 
             params.alpha_right  // Move right penalty
         } else { 
            params.alpha_left  // Move Left Reward
         };

         // Row congestion penalty (the more times the current row is used, the greater the penalty)
         let row_crowding = dynamic_beta * (row_usage[row] as f32);

         // Comprehensive cost calculation
         let delta_y = (block.y - (params.origin_y + row as f32 * params.step_y)).abs();
         let cost = delta_y + alpha * delta_x.abs() + row_crowding;

         if cost < best_cost {
             best_row = row;
             best_cost = cost;
             best_x = left;
         }
        }

        // Verify no overlap
        for r in best_row..best_row + block_rows {
            assert!(
                best_x + block.w <= params.origin_x + params.grid_x as f32 * params.step_x,
                "Placement would exceed right boundary"
            );
        }

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

