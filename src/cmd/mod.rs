pub mod list;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about)]
pub struct Args {
    #[arg(
        short,
        long,
        env,
        default_value_t = String::from("."),
        help = "The directory containing the UMLS files"
    )]
    pub dir: String,
}
