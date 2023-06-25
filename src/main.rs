use std::path::PathBuf;

use clap::Parser;

use nekolib_bundle::{
    library::{Library, LIB_PATH_DEFAULT},
    source::Source,
};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about)]
struct Args {
    #[arg(short, long, help = format!("Library path [{LIB_PATH_DEFAULT}]"))]
    lib_path: Option<PathBuf>,
    source_path: PathBuf,
}

impl Args {
    fn lib_path(&self) -> PathBuf {
        self.lib_path.clone().unwrap_or_else(|| {
            let home = home::home_dir().unwrap();
            LIB_PATH_DEFAULT.replace("~", home.to_str().unwrap()).into()
        })
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let source = Source::new(std::fs::read_to_string(&args.source_path)?);
    let library = Library::traverse(&args.lib_path())?;

    println!("{}", library.bundle(&source));

    Ok(())
}
