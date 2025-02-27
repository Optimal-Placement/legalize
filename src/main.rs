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

    let lp;
    if arguments.file.is_some() {
        lp = legalize::legalize::load(&arguments.file.unwrap());
    } else {
        println!("Must specify an input file");
        return;
    }

    let mut legal = Vec::new();
    if arguments.tetris {
        legal = legalize::legalize::tetris::legalize(&lp);
    }
    if arguments.hcwt {
        legal = legalize::legalize::hcwt_legal::legalize(&lp);
    }
    
    if arguments.postscript.is_some() {
        lp.postscript(&arguments.postscript.unwrap(), legal);
    }


}
