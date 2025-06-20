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


pub fn legalize(lp: &LegalProblem) -> Vec<LegalPosition> {
    // println!("HCWT placement legalizer");

    // let mut blocks = lp.blocks.clone();
    // let params = &lp.params;

    // // Sort blocks by their preferred Y position
    // blocks.sort_by(|a, b| a.y.partial_cmp(&b.y).unwrap());

    // // Initialize HCwT problem
    // // let mut hcwt_prob = hcwt_r::HCwT::new();

    // let mut hcwt_problem = hcwt_r::HCwTProblem::new();

    // // Add blocks to the HCwT problem
    // for block in &blocks {
    //     hcwt_problem.add_block(block.tag, block.w, block.h, block.x, block.y);
    // }

    // // Set HCwT parameters
    // hcwt_problem.set_row_height(params.step_y);
    // hcwt_problem.set_origin(params.origin_x, params.origin_y);

    // // Define the generate callback
    // hcwt_problem.set_generate_callback(|r1, r2, w| {
    //     let new_r1 = r1 + w;
    //     let new_r2 = r2 + w;
    //     vec![(new_r1, r2), (r1, new_r2)]
    // });

    // // Define the filter callback
    // hcwt_problem.set_filter_callback(|r1, r2| {
    //     // Exclude solutions where row lengths differ significantly
    //     if (r1 - r2).abs() > params.step_x * 2.0 {
    //         return false;
    //     }
    //     true
    // });

    // // Solve the HCwT problem
    // let solution = hcwt_problem.solve();

    // // Convert the solution to legal positions
    // let mut legal_positions = Vec::new();
    // for (tag, x, y) in solution {
    //     legal_positions.push(LegalPosition {
    //         block: tag,
    //         x,
    //         y,
    //     });
    // }

    // legal_positions
    Vec::new()
}