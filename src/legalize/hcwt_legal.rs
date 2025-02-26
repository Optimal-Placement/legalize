use bookshelf_r::bookshelf::BookshelfCircuit;
use hcwt_r;

use super::{LegalPosition,LegalBlock,LegalParams};

pub fn legalize(blocks: &Vec<LegalBlock>, params: &LegalParams) -> Vec<LegalPosition> {
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
}