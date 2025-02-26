// Legalizers will move LegalBlocks into legal positions,
// returning a vector of LegalPosition indicating how things
// should move.  
//
// Input to legalizer is a vector of LegalBlocks -- with the tag
// being used to refer back to the parent data structure (a subset
// of cells from a BookshelfCircuit, for example).
//
pub mod tetris;
pub mod hcwt_legal;

use bookshelf_r::bookshelf::BookshelfCircuit;

#[derive(Copy,Clone,Eq,PartialEq)]
pub enum LegalKind {
    Tetris,
    HCwT,
}

#[derive(Copy,Clone)]
pub struct LegalPosition {
    pub block: usize,  // Refers to the index of a LegalBlock
    pub x: f32, // Legalized position XY, lower left corner
    pub y: f32,
}

// Convert 
#[derive(Copy,Clone)]
pub struct LegalBlock {
    pub tag: usize, // Refers to a parent data structure (Bookshelf cell for example)
    pub x: f32, // Preferred X and Y location, lower left corner
    pub y: f32,
    pub h: f32, // Height and width of the block
    pub w: f32,
}

pub struct LegalParams {
    pub grid_x: usize,
    pub grid_y: usize,
    pub origin_x: f32,
    pub origin_y: f32,
    pub step_x: f32,
    pub step_y: f32,
}

pub struct LegalProblem {
    pub blocks: Vec<LegalBlock>,
    pub params: LegalParams,
}

pub fn legalize_circuit(bc: &mut BookshelfCircuit, kind: LegalKind) {
    let mut blocks = Vec::new();
    
    for c in 0..bc.cells.len() {
        if !bc.cells[c].terminal {
            blocks.push(LegalBlock{
                tag: c,
                x: bc.cellpos[c].x,
                y: bc.cellpos[c].y,
                h: bc.cells[c].h,
                w: bc.cells[c].w,
            });
        }
    }

    let b = bc.rows[0].bounds;

    let width = b.urx - b.llx;
    let height = b.ury - b.lly;
    
    let params = LegalParams{
        grid_x: (width/bc.rows[0].site_spacing) as usize,
        grid_y: bc.rows.len(),
        origin_x: bc.rows[0].bounds.llx,
        origin_y: bc.rows[0].bounds.lly,
        step_x: bc.rows[0].site_spacing,
        step_y: height,
    };

    println!("Legalize {} blocks\nIn space: {}", blocks.len(), params);


    if kind == LegalKind::Tetris {
        let result = tetris::legalize(&blocks, &params);
    }

    if kind == LegalKind::HCwT {
        let result = hcwt_legal::legalize(&blocks, &params);
    }
    
}

use std::fmt;

impl fmt::Display for LegalParams {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} sites {} rows  origin {},{}   site width: {} row height {}", self.grid_x, self.grid_y, self.origin_x, self.origin_y,
    self.step_x, self.step_y)
    }
}