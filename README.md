# JPK23 - JPK_V7(1)/JPK_V7(2) to JPK_V7(3) Converter

A blazing-fast, stream-based command-line tool written in Rust to convert legacy version 1 (`JPK_VAT(3)` schema 1-1) and version 2 (`JPK_V7M(2)` / `JPK_V7K(2)`) XML files to the mandatory output JPK_V7(3) format required by the Polish Ministry of Finance (KAS) from February 2026.

## Features
- **Schema Upgrade:** Automatically updates XML namespaces, `kodSystemowy`, and root attributes.
- **M/K Variant Control:** Automatically detects if the source is Monthly (M) or Quarterly (K) and allows explicit overrides via CLI flags.
- **Automated Control Totals:** Recalculates `PodatekNalezny` and `PodatekNaliczony` based on mandatory V3 summation rules, corrects discrepancies in `SprzedazCtrl`/`ZakupCtrl`, and warns the user.
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
jpk23 --in res/v2_sample.xml --out output_v3.xml -u 1438
```

### Options

- `-i, --in [FILE]`: Specify the input JPK_V7 XML file. Supports Version 1 and 2. Reads from `stdin` if omitted.
- `-o, --out [FILE]`: Specify the output JPK_V7(3) XML file. Writes to `stdout` if omitted.
- `-u, --urzad [CODE]`: Set the `KodUrzedu` (Tax Office Code) in the header (mandatory in V3).
- `-m, --v7m`: Force output variant to JPK_V7M (Monthly).
- `-k, --v7k`: Force output variant to JPK_V7K (Quarterly).
- `-n, --namespace [PREFIX]`: Set a custom namespace prefix (e.g. `jpk`). Use without value to strip prefixes entirely.
- `-f, --force`: Force overwrite the output file without prompting.
- `-h, --help`: Display help information.
- `-v, --version`: Print version and license info.

## Examples

**Convert with Tax Office Code:**
```sh
jpk23 -i source.xml -o converted.xml -u 1438
```

**Force Quarterly Output:**
```sh
jpk23 -i source.xml -o converted.xml -k
```

**Strip Namespace Prefixes:**
```sh
jpk23 -i source.xml -o converted.xml -n
```

**Stream processing (ideal for scripts):**
```sh
cat source.xml | jpk23 --in --out > converted.xml
```

## Control Total Validation
The tool automatically recalculates control sums during processing. If the values in the source file differ from the calculated sums, a warning is displayed:
`Warning: Corrected ZakupCtrl/PodatekNaliczony from 8023.4 to 200302.33`

## License
[MIT](LICENSE)
