mod jpk23;

use anyhow::{Context, Result};
use clap::{CommandFactory, Parser};
use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Converts JPK_V7(1) or JPK_V7(2) to JPK_V7(3) format.
#[derive(Parser, Debug)]
#[command(author, about, long_about = None, disable_version_flag = true)]
struct Args {
    /// Print version
    #[arg(short = 'v', long = "version")]
    version: bool,
    /// Input JPK_V7 XML file (supports Version 1 `JPK_VAT(3)` and Version 2 `JPK_V7M(2)/JPK_V7K(2)`). If missing, reads from standard input.
    #[arg(short = 'i', long = "in", value_name = "FILE", num_args = 0..=1, default_missing_value = "")]
    input: Option<String>,

    /// Output JPK_V7(3) XML file. If missing, writes to standard output.
    #[arg(short = 'o', long = "out", value_name = "FILE", num_args = 0..=1, default_missing_value = "")]
    output: Option<String>,

    /// Set the namespace prefix of the output document (e.g. 'jpk'). If switch is provided without a value, strips the namespace prefix entirely.
    #[arg(short = 'n', long = "namespace", value_name = "PREFIX", num_args = 0..=1, default_missing_value = "")]
    namespace: Option<String>,

    /// Force overwrite the output file without asking
    #[arg(short = 'f', long = "force")]
    force: bool,

    /// Set the KodUrzedu (Tax Office Code) for the Naglowek (mandatory in V3).
    #[arg(short = 'u', long = "urzad", value_name = "CODE")]
    urzad: Option<String>,

    /// Force output variant to JPK_V7M (Monthly). Default if undetermined.
    #[arg(short = 'm', long = "v7m")]
    v7m: bool,

    /// Force output variant to JPK_V7K (Quarterly).
    #[arg(short = 'k', long = "v7k")]
    v7k: bool,
}

fn main() -> Result<()> {
    if std::env::args().len() == 1 {
        let mut cmd = Args::command();
        // Extract version manually if needed
        eprintln!("JPK23 Version {} License: MIT\n", VERSION);
        let _ = cmd.print_help();
        std::process::exit(0);
    }

    let args = Args::parse();

    if args.version {
        println!("jpk23 {}", VERSION);
        std::process::exit(0);
    }

    let input_reader: Box<dyn BufRead> = match args.input.as_deref() {
        Some(path_str) if !path_str.is_empty() => {
            let file = File::open(path_str).with_context(|| format!("Failed to open {:?}", path_str))?;
            Box::new(BufReader::new(file))
        }
        _ => Box::new(BufReader::new(io::stdin())),
    };

    let mut output_writer: Box<dyn Write> = match args.output.as_deref() {
        Some(path_str) if !path_str.is_empty() => {
            let path = std::path::Path::new(path_str);
            if path.exists() && !args.force {
                eprint!("File {:?} already exists. Overwrite? [y/N]: ", path);
                io::stderr().flush()?;
                let mut buf = String::new();
                io::stdin().read_line(&mut buf)?;
                if !buf.trim().eq_ignore_ascii_case("y") {
                    eprintln!("Operation cancelled.");
                    std::process::exit(0);
                }
            }
            let file = File::create(path).with_context(|| format!("Failed to create {:?}", path))?;
            Box::new(file)
        }
        _ => Box::new(io::stdout()),
    };

    let explicit_variant = if args.v7k {
        jpk23::FormVariant::K
    } else if args.v7m {
        jpk23::FormVariant::M
    } else {
        jpk23::FormVariant::Unknown
    };

    jpk23::process_jpk(input_reader, &mut output_writer, args.namespace.clone(), args.urzad.clone(), explicit_variant)
}
