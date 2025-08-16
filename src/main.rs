use argh::FromArgs;
#[derive(FromArgs)]
/// Placement legalization
struct Args {
    /// tetris legalization
    #[argh(switch, short = 't')]
    tetris: bool,

    /// hcwt legalization
    #[argh(switch, short = 'h')]
    hcwt: bool,

    /// rowfill legalization
    #[argh(switch, short = 'r')]
    rowfill: bool,

    /// row number adjustment
    #[argh(option, short = 'd')]
    delta_row: Option<i32>,

    /// file to load
    #[argh(option, short = 'f')]
    file: Option<String>,

    /// output PL file
    #[argh(option, short = 'o')]
    output: Option<String>,

    /// postScript output file
    #[argh(option, short = 'P')]
    postscript: Option<String>,
}

fn main() {
    println!("Stand-alone placement legalizer");
    let arguments: Args = argh::from_env();

    let mut lp;
    if arguments.file.is_some() {
        lp = legalize::legalize::load(&arguments.file.unwrap());
    } else {
        println!("Must specify an input file");
        return;
    }

    if arguments.delta_row.is_some() {
        println!("Adjust number of rows by {}", arguments.delta_row.unwrap());
        lp.params.grid_y = (lp.params.grid_y as i32 + arguments.delta_row.unwrap()) as usize;
        lp.rescale();
    }

    let mut legal = Vec::new();
    if arguments.tetris {
        legal = legalize::legalize::tetris::legalize(&lp);
    }
    if arguments.hcwt {
        legal = legalize::legalize::hcwt_legal::legalize(&lp);
    }
    if arguments.rowfill {
        legal = legalize::legalize::rowfill::legalize(&lp);
    }

    if arguments.postscript.is_some() {
        lp.postscript(&arguments.postscript.unwrap(), &legal);
    }
}
