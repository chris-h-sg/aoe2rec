use clap::Parser;

#[derive(Parser)]
struct Args {
    // Path to the aoe2record file
    file: std::path::PathBuf,
    #[arg(short, long)]
    summary: bool,
    // Output Build, Queue and Delete actions with decoded fields
    #[arg(short, long, conflicts_with = "summary")]
    typed: bool,
}
fn main() {
    let args = Args::parse();
    if !args.file.is_file() {
        println!("File not found");
        std::process::exit(-1);
    }
    let parsed_game = aoe2rec::Savegame::from_file(&args.file).unwrap();
    if args.typed {
        println!(
            "{}",
            serde_json::to_string_pretty(&parsed_game.get_typed()).unwrap()
        );
    } else if !args.summary {
        println!("{}", serde_json::to_string_pretty(&parsed_game).unwrap());
    } else {
        println!(
            "{}",
            serde_json::to_string_pretty(&parsed_game.get_summary()).unwrap()
        );
    }
}
