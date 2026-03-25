mod jpk23;

use anyhow::{Context, Result};
use clap::{CommandFactory, Parser};
use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn format_currency(value: f64) -> String {
    let is_negative = value < 0.0;
    let s = format!("{:.2}", value.abs());
    let parts: Vec<&str> = s.split('.').collect();
    let int_part = parts[0];
    let frac_part = parts[1];

    let mut result = String::new();
    let mut count = 0;
    for c in int_part.chars().rev() {
        if count > 0 && count % 3 == 0 {
            result.push(' ');
        }
        result.push(c);
        count += 1;
    }
    let grouped_int: String = result.chars().rev().collect();
    let sign = if is_negative { "-" } else { "" };
    format!("{}{}.{}", sign, grouped_int, frac_part)
}

/// Converts JPK_V7(1) or JPK_V7(2) to JPK_V7(3) format.
#[derive(Parser, Debug)]
#[command(author, about, long_about = None, disable_version_flag = true)]
struct Args {
    /// Print version
    #[arg(short = 'v', long = "version", action = clap::ArgAction::SetTrue, overrides_with = "version")]
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

    /// Pretty print XML output (indentation)
    #[arg(short = 'p', long, action = clap::ArgAction::SetTrue, overrides_with = "pretty")]
    pretty: bool,

    /// Suppress XML output (useful for pure validation or summary)
    #[arg(short = 'q', long, action = clap::ArgAction::SetTrue, overrides_with = "quiet")]
    quiet: bool,

    /// Print summary at the end
    #[arg(short = 's', long, action = clap::ArgAction::SetTrue, overrides_with = "summary")]
    summary: bool,

    /// Set the KodUrzedu (Tax Office Code) for the Naglowek (mandatory in V3).
    #[arg(short = 'u', long = "urzad", value_name = "CODE")]
    urzad: Option<String>,

    /// Force output variant to JPK_V7M (Monthly). Default if undetermined.
    #[arg(short = 'm', long = "v7m", action = clap::ArgAction::SetTrue, overrides_with = "v7m")]
    v7m: bool,

    /// Force output variant to JPK_V7K (Quarterly).
    #[arg(short = 'k', long = "v7k", action = clap::ArgAction::SetTrue, overrides_with = "v7k")]
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

    let output_stream: Box<dyn Write> = if args.quiet {
        Box::new(io::sink())
    } else {
        match args.output.as_deref() {
            Some(path_str) if !path_str.is_empty() => {
                let path = std::path::Path::new(path_str);
                if path.exists() {
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
        }
    };

    let explicit_variant = if args.v7k {
        jpk23::FormVariant::K
    } else if args.v7m {
        jpk23::FormVariant::M
    } else {
        jpk23::FormVariant::Unknown
    };

    let mut output_writer = if args.pretty {
        quick_xml::Writer::new_with_indent(output_stream, b' ', 2)
    } else {
        quick_xml::Writer::new(output_stream)
    };

    let stats = jpk23::process_jpk(input_reader, &mut output_writer, args.namespace.clone(), args.urzad.clone(), explicit_variant)?;

    if args.summary {
        // Print summary in green
        eprintln!("\x1b[32m");
        eprintln!("=============================================================");
        eprintln!("                     CONVERSION SUMMARY");
        eprintln!("=============================================================");
        eprintln!(" Original Version: {:?}", stats.original_version);
        eprintln!(" Taxpayer NIP:     {}", stats.taxpayer_nip.as_deref().unwrap_or("N/A"));
        eprintln!("-------------------------------------------------------------");
        eprintln!(" SALES (PLN):");
        eprintln!("   Records:        {:>41}", stats.sales_count);
        eprintln!("   Total Net:      {:>41}", format_currency(stats.total_sales_base));
        eprintln!("   Total VAT:      {:>41}", format_currency(stats.total_sales_vat));
        if let Some((val, row)) = stats.max_sales_vat {
            let row_str = format!("({:>})", row);
            eprintln!("   Max VAT (Row):  {:>22} {:>18}", row_str, format_currency(val));
        }
        if let Some((val, row)) = stats.min_sales_vat {
            let row_str = format!("({:>})", row);
            eprintln!("   Min VAT (Row):  {:>22} {:>18}", row_str, format_currency(val));
        }
        eprintln!("   Rate Breakdown:                Net               VAT");
        eprintln!("       23%         {:>22} {:>18}", format_currency(stats.breakdown.base_23), format_currency(stats.breakdown.vat_23));
        eprintln!("        8%         {:>22} {:>18}", format_currency(stats.breakdown.base_8), format_currency(stats.breakdown.vat_8));
        eprintln!("        5%         {:>22} {:>18}", format_currency(stats.breakdown.base_5), format_currency(stats.breakdown.vat_5));
        eprintln!("     Other         {:>22} {:>18}", format_currency(stats.breakdown.base_other), format_currency(stats.breakdown.vat_other));
        eprintln!("-------------------------------------------------------------");
        eprintln!(" PURCHASES (PLN):");
        eprintln!("   Records:        {:>41}", stats.purchase_count);
        eprintln!("   Total Net:      {:>41}", format_currency(stats.total_purchase_base));
        eprintln!("   Total VAT:      {:>41}", format_currency(stats.total_purchase_vat));
        if let Some((val, row)) = stats.max_purchase_vat {
            let row_str = format!("({:>})", row);
            eprintln!("   Max VAT (Row):  {:>22} {:>18}", row_str, format_currency(val));
        }
        if let Some((val, row)) = stats.min_purchase_vat {
            let row_str = format!("({:>})", row);
            eprintln!("   Min VAT (Row):  {:>22} {:>18}", row_str, format_currency(val));
        }
        eprintln!("-------------------------------------------------------------");
        let diff = stats.total_sales_vat - stats.total_purchase_vat;
        eprintln!(" FINAL VAT BALANCE (PLN): {:>34}", format_currency(diff));
        eprintln!("=============================================================");
        eprintln!("\x1b[0m");
    }

    Ok(())
}
