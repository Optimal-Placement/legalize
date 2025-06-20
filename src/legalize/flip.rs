use bookshelf_r::bookshelf::BookshelfCircuit;
use super::{LegalBlock, LegalParams, LegalPosition, LegalProblem};

pub fn legalize(lp: &LegalProblem) -> Vec<LegalPosition> {
    println!("Tetris placement legalizer with flip-rightalign-flip");

    let mut blocks = lp.blocks.clone();
    let params = &lp.params;
    let max_x = params.origin_x + params.grid_x as f32 * params.step_x;

    // mirror flip all blocks horizontally
    let flipped_blocks: Vec<LegalBlock> = blocks.iter()
        .map(|b| {
            // Mirror flip around the center of the placement area
            let flipped_x = max_x - b.x - b.w;
            LegalBlock {
                x: flipped_x,
                ..b.clone()
            }
        })
        .collect();

    // Sort flipped blocks by their new X position (now right-to-left)
    let mut sorted_blocks = flipped_blocks.clone();
    sorted_blocks.sort_by(|a, b| b.x.partial_cmp(&a.x).expect("Could not compare")); // Reverse sort

    // Initialize right edge for flipped world
    let mut right_edges = vec![max_x; params.grid_y];
    let mut row_usage = vec![0usize; params.grid_y];
    let mut flipped_positions = Vec::new();

    const BETA: f32 = 0.5;

    // Place blocks in flipped world with right alignment
    for block in &sorted_blocks {
        let block_rows = (block.h / params.step_y).ceil() as usize;
        let block_rows = block_rows.max(1);
        
        let best_row = ((block.y - params.origin_y) / params.step_y).floor() as usize;
        let search_radius = (5 * params.grid_y / 100).max(5);
        let low_row = best_row.saturating_sub(search_radius);
        let high_row = (best_row + search_radius).min(params.grid_y.saturating_sub(block_rows));

        let mut best_row = best_row;
        let mut best_cost = f32::MAX;
        let mut best_x = max_x - block.w;

        for row in low_row..=high_row {
            let row_range = row..(row + block_rows);
            let right = row_range.clone()
                .map(|r| right_edges.get(r).unwrap_or(&max_x))
                .fold(max_x, |a, &b| a.min(b));

            let dynamic_beta = BETA * (1.0 + row_usage[row] as f32 / 10.0);
            let delta_x = block.x - (right - block.w);
            let alpha = if delta_x > 0.0 {
                params.alpha_left
            } else {
                params.alpha_right
            };

            let placed_y = params.origin_y + row as f32 * params.step_y;
            let delta_y = (block.y - placed_y).abs() + 
                         (block.h - (block_rows as f32 * params.step_y)).abs() * 0.1;

            let cost = delta_y + alpha * delta_x.abs() + dynamic_beta * row_usage[row] as f32;

            if cost < best_cost {
                best_row = row;
                best_cost = cost;
                best_x = right - block.w;
            }
        }

        assert!(
            best_row + block_rows <= params.grid_y,
            "Vertical placement out of bounds"
        );

        flipped_positions.push(LegalPosition {
            block: block.tag,
            x: best_x,
            y: params.origin_y + best_row as f32 * params.step_y,
        });

        for r in best_row..best_row + block_rows {
            right_edges[r] = best_x;
            row_usage[r] += 1;
        }
    }

    // Flip all positions back to original orientation
    let legal_positions: Vec<LegalPosition> = flipped_positions.into_iter()
        .map(|pos| {
            let original_block = blocks.iter().find(|b| b.tag == pos.block).unwrap();
            let new_x = max_x - (pos.x + original_block.w);
            LegalPosition {
                block: pos.block,
                x: new_x,
                y: pos.y,
            }
        })
        .collect();

    legal_positions
}