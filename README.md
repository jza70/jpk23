# JPK23 - JPK_V7(1)/JPK_V7(2) to JPK_V7(3) Converter

A blazing-fast, stream-based command-line tool written in Rust to convert legacy version 1 (`JPK_VAT(3)` schema 1-1) and version 2 (`JPK_V7M(2)` / `JPK_V7K(2)`) XML files to the mandatory output JPK_V7(3) format required by the Polish Ministry of Finance (KAS) from February 2026. It also supports native JPK_V7(3) processing for pretty-printing, variant switching, and generating conversion summaries.

## Features

- **Schema Upgrade:** Automatically updates XML namespaces, `kodSystemowy`, and root attributes.
- **M/K Variant Control:** Automatically detects if the source is Monthly (M) or Quarterly (K) and allows explicit overrides via CLI flags.
- **Automated Control Totals:** Recalculates `PodatekNalezny` and `PodatekNaliczony` based on mandatory V3 summation rules, warns about discrepancies in `SprzedazCtrl`/`ZakupCtrl`, and saves the warning as a comment in the output file next to the problematic element.
- **Summary:** Optional detailed green summary at the end of conversion (enabled with `-s`).
- **Row Management:** Automatically updates `LiczbaWierszySprzedazy` and `LiczbaWierszyZakupow` counters.
- **Strict Compliance:** Injects mandated choice tags (`<BFK>1</BFK>` or `<DI>1</DI>`) and ensures strict element sequence in `Naglowek`.
- **Podmiot1 Modernization:** Nests legacy flat structures into compliant `OsobaNiefizyczna`/`OsobaFizyczna` blocks and ensures mandatory `Email` presence.
- **Memory Efficient:** Processes large JPK structures directly as an event stream (`quick-xml`), handling files natively without loading the entire DOM into RAM.

## Installation

Ensure you have Rust and Cargo installed, then build the binary:

```sh
git clone https://github.com/jza70/jpk23.git
cd jpk23
cargo build --release
```

The executable will be generated at `target/release/jpk23.exe`.

## Usage

```sh
jpk23 --in res/v2_sample.xml --out output_v3.xml -u 1438 -s
```

### Options

- `-i, --in [FILE]`: Specify the input JPK_V7 XML file. Supports Version 1, 2, and 3. Reads from `stdin` if omitted.
- `-o, --out [FILE]`: Specify the output JPK_V7(3) XML file. Writes to `stdout` if omitted.
- `-u, --urzad [CODE]`: Set the `KodUrzedu` (Tax Office Code) in the header (mandatory in V3).
- `-m, --v7m`: Force output variant to JPK_V7M (Monthly).
- `-k, --v7k`: Force output variant to JPK_V7K (Quarterly).
- `-n, --namespace [PREFIX]`: Set a custom namespace prefix (e.g. `jpk`). Use without value to strip prefixes entirely.
- `-p, --pretty`: Format the output XML with indentation (pretty print).
- `-q, --quiet`: Suppress XML output (useful for pure validation or summary).
- `-s, --summary`: Print summary at the end of the conversion (in green to `stderr`).
- `-h, --help`: Display help information.
- `-v, --version`: Print version and license info.

### Flag Clustering

`jpk23` supports joining single-letter options (flags) into a single block. If an option requires a value (like `-i`, `-o`, or `-u`), it must be the **last** letter in the cluster.

#### Example: Join Quiet, Summary, and Input

```sh
jpk23 -qsi input.xml
```

(This is equivalent to `jpk23 -q -s -i input.xml`)

## Examples

### Convert with Tax Office Code and Summary

```sh
jpk23 -i source.xml -o converted.xml -u 1438 -s
```

### Quiet Mode (Summary only)

```sh
jpk23 -i source.xml -q -s
```

### Force Quarterly Output with Pretty Print

```sh
jpk23 -i source.xml -o converted.xml -k -p
```

### Strip Namespace Prefixes

```sh
jpk23 -i source.xml -o converted.xml -n
```

## Control Total Validation

The tool automatically recalculates control sums during processing. If the values in the source file differ from the calculated sums, a warning is displayed and the comment is added to the output file.

## Summary

With --summary or -s flag, the tool prints a summary of the conversion at the end. It shows the original version of the file, the taxpayer's NIP, the number of sales and purchase records, the total sales and purchase base and VAT, the maximum and minimum sales and purchase VAT, and the breakdown of sales and purchase VAT by rate.

```text
=============================================================
                     CONVERSION SUMMARY
=============================================================
 Original Version: V2
 Taxpayer NIP:     5550000000
-------------------------------------------------------------
 SALES (PLN):
   Records:                                                2
   Total Net:                                   1 239 567.89
   Total VAT:                                     284 350.62
   Max VAT (Row):                     (1)         283 950.62
   Min VAT (Row):                     (2)             400.00
   Rate Breakdown:                Net               VAT
       23%                     1 234 567.89       283 950.62
        8%                         5 000.00           400.00
        5%                             0.00             0.00
     Other                             0.00             0.00
-------------------------------------------------------------
 PURCHASES (PLN):
   Records:                                                1
   Total Net:                                         200.00
   Total VAT:                                          46.00
   Max VAT (Row):                     (1)              46.00
   Min VAT (Row):                     (1)              46.00
-------------------------------------------------------------
 FINAL VAT BALANCE (PLN):                         284 304.62
=============================================================
```

## Changelog

### [1.0.1] - 2026-03-25

- **Native JPK_V7(3) Support:** Added ability to process already-converted V3 files as input.
- **Quiet Mode:** Added `-q, --quiet` flag to suppress XML output (useful for validation/summary).
- **Clustering Support:** Improved CLI experience to allow joining single-letter options (e.g. `-qsi`).
- **Robust Flags:** Standardized boolean flags to be idempotent and allow multiple occurrences.

### [1.0.0] - 2026-03-24

- **Initial Release:** Complete JPK_V7(1) and JPK_V7(2) to JPK_V7(3) conversion logic.
- **Audit Summary:** Implementation of professional green terminal summary with precise alignment.
- **Symmetry:** Symmetrical layout for summary statistics (61-character width).
- **Namespace Control:** Added support for stripping or prefixing namespaces.

## License

[MIT](LICENSE)
