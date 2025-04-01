use bookshelf_r::bookshelf::BookshelfCircuit;

use super::{LegalBlock, LegalParams, LegalPosition, LegalProblem};

/*pub fn legalize(lp: &LegalProblem) -> Vec<LegalPosition> {
    println!("Tetris placement legalizer");
    Vec::new()
}
*/

pub fn legalize(lp: &LegalProblem) -> Vec<LegalPosition> {
    println!("Tetris placement legalizer");

    let mut blocks = lp.blocks.clone();
    let params = &lp.params;

    // Sort blocks by their preferred X position
    blocks.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap());

    //Initialize left edge
    let mut legal_positions = Vec::new();
    let mut left_edges = vec![params.origin_x; params.grid_y];

    //Go through each block and find the best place to put it
    for block in &blocks {
        let best_row = ((block.y - params.origin_y) / params.step_y).round() as usize;
        let block_rows = (block.h / params.step_y).ceil() as usize;

        let low_row = best_row.saturating_sub(3);
        let high_row = (best_row + 3).min(params.grid_y - block_rows);

        //Find the location of minimum displacement near the optimal row
        let mut best_row = best_row;
        let mut best_displacement = f32::MAX;
        let mut best_edge = params.origin_x;

        for row in low_row..=high_row {
            let mut left = left_edges[row];
            for r in row..row + block_rows {
                if left_edges[r] > left {
                    left = left_edges[r];
                }
            }

            let displacement = left + (block.y - (params.origin_y + row as f32 * params.step_y)).abs();

            if displacement < best_displacement {
                best_row = row;
                best_displacement = displacement;
                best_edge = left;
            }
        }

        let legal_x = best_edge;
        let legal_y = params.origin_y + best_row as f32 * params.step_y;

        legal_positions.push(LegalPosition {
            block: block.tag,
            x: legal_x,
            y: legal_y,
        });

        //Update the left edge of each row
        for r in best_row..best_row + block_rows {
            left_edges[r] = legal_x + block.w;
        }
    }

    legal_positions
}