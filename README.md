# JPK23 - JPK_V7(1)/JPK_V7(2) to JPK_V7(3) Converter

A blazing-fast, stream-based command-line tool written in Rust to convert legacy version 1 (`JPK_VAT(3)` schema 1-1) and version 2 (`JPK_V7M(2)` / `JPK_V7K(2)`) XML files to the mandatory output JPK_V7(3) format required by the Polish Ministry of Finance (KAS) from February 2026.

## Features
- **Schema Upgrade:** Automatically updates XML namespaces, `kodSystemowy`, and root attributes.
- **M/K Variant Control:** Automatically detects if the source is Monthly (M) or Quarterly (K) and allows explicit overrides via CLI flags.
- **Automated Control Totals:** Recalculates `PodatekNalezny` and `PodatekNaliczony` based on mandatory V3 summation rules, corrects discrepancies in `SprzedazCtrl`/`ZakupCtrl`, and warns the user.
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

- `-i, --in [FILE]`: Specify the input JPK_V7 XML file. Supports Version 1 and 2. Reads from `stdin` if omitted.
- `-o, --out [FILE]`: Specify the output JPK_V7(3) XML file. Writes to `stdout` if omitted.
- `-u, --urzad [CODE]`: Set the `KodUrzedu` (Tax Office Code) in the header (mandatory in V3).
- `-m, --v7m`: Force output variant to JPK_V7M (Monthly).
- `-k, --v7k`: Force output variant to JPK_V7K (Quarterly).
- `-n, --namespace [PREFIX]`: Set a custom namespace prefix (e.g. `jpk`). Use without value to strip prefixes entirely.
- `-p, --pretty`: Format the output XML with indentation (pretty print).
- `-s, --summary`: Print summary at the end of the conversion (in green to `stderr`).
- `-h, --help`: Display help information.
- `-v, --version`: Print version and license info.

## Examples

**Convert with Tax Office Code and Summary:**

```sh
jpk23 -i source.xml -o converted.xml -u 1438 -s
```

**Force Quarterly Output with Pretty Print:**

```sh
jpk23 -i source.xml -o converted.xml -k -p
```

**Strip Namespace Prefixes:**

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
       23%                     1 234 567.89         283 950.62
        8%                         5 000.00             400.00
        5%                             0.00               0.00
     Other                             0.00               0.00
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

## License

[MIT](LICENSE)
