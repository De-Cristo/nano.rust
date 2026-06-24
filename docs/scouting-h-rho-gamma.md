# Run-3 Scouting H -> rho gamma Demo

This page documents the `nano-io` `scouting_h_rho_gamma` example. It is a
Run-3 scouting-oriented `H -> rho gamma` demonstrator over local
NanoAODv15-like signal MC. It is not a full scouting analysis, not a trigger
efficiency study, and not a reproduction of a published `H -> rho/phi/K*
gamma` analysis.

The current goal is reproducibility and semantic clarity: read one local ROOT
file, build a transparent photon plus `pi+ pi-` candidate, and print a compact
cutflow and candidate summary.

## Input Convention

The example accepts a local ROOT file path as its first positional argument:

```bash
cargo run -p nano-io --example scouting_h_rho_gamma -- /path/to/input.root
```

If no positional path is given, it falls back to:

```bash
NANO_SCOUTING_HRHOGAMMA_FILE=/path/to/input.root
```

An optional second positional argument limits the number of processed events:

```bash
cargo run -p nano-io --example scouting_h_rho_gamma -- /path/to/input.root 100
```

An optional third positional argument supplies a config path:

```bash
cargo run -p nano-io --example scouting_h_rho_gamma -- /path/to/input.root 100 configs/scouting/h_rho_gamma.toml
```

Optional CSV and ROOT output writes every accepted H candidate:

```bash
cargo run -p nano-io --example scouting_h_rho_gamma -- /path/to/input.root 100 configs/scouting/h_rho_gamma.toml --csv candidates.csv --root candidates.root
```

If no config path is supplied, the example loads
`configs/scouting/h_rho_gamma.toml` by default. If that default file is absent,
it falls back to the built-in `HToRhoGammaCuts::zcountinghlt_naive()` values.
If an explicit config path is supplied and cannot be read or validated, the
example fails clearly instead of silently falling back.

The committed code and scripts intentionally do not hardcode personal absolute
paths.

## Demo Script

The wrapper script checks the input file exists and then runs the same example:

```bash
scripts/demo_scouting_hrhogamma.sh /path/to/input.root
```

With an event limit:

```bash
scripts/demo_scouting_hrhogamma.sh /path/to/input.root 100
```

With an explicit config:

```bash
scripts/demo_scouting_hrhogamma.sh /path/to/input.root 100 configs/scouting/h_rho_gamma.toml
```

With CSV output:

```bash
scripts/demo_scouting_hrhogamma.sh /path/to/input.root 100 configs/scouting/h_rho_gamma.toml --csv candidates.csv
```

Or with the environment fallback:

```bash
NANO_SCOUTING_HRHOGAMMA_FILE=/path/to/input.root scripts/demo_scouting_hrhogamma.sh
```

## Expected Output Sections

The example prints plain text. The important sections are:

- `input`: the ROOT file path used for this run.
- `max_events`: either `all` or the supplied event limit.
- `cut_source`: the TOML config table used for cut values, or the built-in
  fallback if the default config file is absent.
- `candidate_output`: either `none` or the CSV path supplied with `--csv`.
- `branch_schema`: confirms the requested `nano_io::events_chunked` schema was
  built.
- `branch_mapping`: documents the semantic scouting mapping used by this first
  local-file version.
- `constants`: pion, rho, and Higgs reference masses.
- `cuts`: the ZCountingHLT naive baseline cuts used by the example.
- `processed_events`: number of events read.
- `pfcand_pions`: global `+211` and `-211` counts plus average source
  `PFCand_mass` for selected pion-like PFCands.
- `accepted_candidates`: number of events that pass the full naive candidate
  path.
- `cutflow`: event counts after each selection step.
- `first_candidates`: up to 10 candidate summaries with photon, pion, rho, and
  H-candidate kinematics.

By default, the example remains print-only. With `--csv`, the text summary is
still printed and every accepted H candidate is also written to the CSV file.
The printed `first_candidates` section is still capped at 10 candidates.

## CSV Candidate Output

CSV output is candidate-level and contains one row for every accepted H
candidate. It is not an event skim, ROOT ntuple, or histogram file.

The stable columns are:

```text
run
luminosityBlock
event
photon_pt
photon_eta
photon_phi
pi_plus_pt
pi_plus_eta
pi_plus_phi
pi_minus_pt
pi_minus_eta
pi_minus_phi
rho_mass
rho_pt
rho_eta
rho_phi
h_mass
h_pt
h_eta
h_phi
delta_r_pipi
delta_r_gamma_rho
rho_pt_over_photon_pt
```

## CSV Validation and Quick Plots

Stage 8 adds a lightweight validation script that consumes the candidate CSV
from the Rust example and writes a summary plus simple sanity plots. This is a
reproducibility aid for checking the demo output, not a statistical analysis or
physics-category implementation.

First generate a candidate CSV:

```bash
scripts/demo_scouting_hrhogamma.sh /path/to/input.root 100 configs/scouting/h_rho_gamma.toml --csv /tmp/scouting_hrhogamma_candidates.csv
```

Then summarize and plot it:

```bash
python scripts/plot_scouting_hrhogamma_csv.py /tmp/scouting_hrhogamma_candidates.csv --outdir /tmp/scouting_hrhogamma_plots
```

The script always writes:

```text
summary.txt
```

The summary includes the input CSV path, candidate row count, unique event
count, duplicate `run/luminosityBlock/event` entries, and min/mean/max for:

```text
h_mass
rho_mass
photon_pt
rho_pt
delta_r_pipi
delta_r_gamma_rho
rho_pt_over_photon_pt
```

If `matplotlib` is available, the script also writes one PNG histogram per
quantity. The Stage 11 production path uses the expanded physics-check set:

```text
h_mass.png
rho_mass.png
photon_pt.png
rho_pt.png
pi_plus_pt.png
pi_minus_pt.png
h_pt.png
delta_r_pipi.png
delta_r_gamma_rho.png
rho_pt_over_photon_pt.png
h_mass_vs_rho_mass.png
photon_pt_vs_h_mass.png
rho_pt_vs_h_mass.png
delta_r_gamma_rho_vs_h_mass.png
```

If `matplotlib` is not installed, `summary.txt` is still written and the script
prints a clear message that plots were skipped. Use `--no-plots` to force
summary-only mode. Use `--max-rows N` to inspect only the first `N` candidate
rows.

## Branch Mapping

The branch catalogue is in `configs/branches/scouting_run3.yaml`. The example reads `[analysis].branch_catalogue` and dynamically resolves semantic aliases:

```text
ScoutingPhoton.count -> nPhoton
ScoutingPhoton.pt    -> Photon_pt
ScoutingPhoton.eta   -> Photon_eta
ScoutingPhoton.phi   -> Photon_phi

ScoutingChargedCandidate.count -> nPFCand
ScoutingChargedCandidate.pt    -> PFCand_pt
ScoutingChargedCandidate.eta   -> PFCand_eta
ScoutingChargedCandidate.phi   -> PFCand_phi
ScoutingChargedCandidate.mass  -> PFCand_mass
ScoutingChargedCandidate.pdgId -> PFCand_pdgId
```

`Photon_mass` is absent in the local files, so photon mass is fixed to zero.
`PFCand_charge` is absent, so charge is inferred from the sign of
`PFCand_pdgId`.

## Baseline Cuts

The baseline values are recorded in
`configs/scouting/h_rho_gamma.toml` under
`[baseline.zcountinghlt_naive]`:

```text
photon_min_pt = 15.0
pi1_min_pt = 5.0
pi2_min_pt = 2.0
max_delta_r_pipi = 0.1
rho_mass_min = 0.3
rho_mass_max = 1.2
min_delta_r_gamma_rho = 1.0
max_delta_r_gamma_rho = 5.0
pion_mass = 0.13957039
rho_mass_target = 0.77526
higgs_mass_reference = 125.0
```

The library keeps these values as built-in fallback values so that the demo
remains self-contained when the default config is absent. At runtime, it loads the same values from the
config by default and prints:

```text
cut_source: configs/scouting/h_rho_gamma.toml [baseline.zcountinghlt_naive]
```

When the default config file is absent, the output instead reports:

```text
cut_source: built-in zcountinghlt_naive fallback
```

## Candidate Algorithm

Per event, the example:

1. Selects the highest-pt photon with `Photon_pt >= photon_min_pt`.
2. Collects PFCands with `abs(PFCand_pdgId) == 211`, nonzero inferred charge,
   and `PFCand_pt >= pi2_min_pt`.
3. Sorts pion candidates by descending pt.
4. Loops unique pairs requiring:
   - leading pion pt >= `pi1_min_pt`
   - subleading pion pt >= `pi2_min_pt`
   - opposite sign
   - `deltaR(pi, pi) < max_delta_r_pipi`
   - rho mass in `[rho_mass_min, rho_mass_max]`
5. Chooses the passing pair closest to `rho_mass_target`.
6. Combines the selected rho with the selected photon.
7. Requires `deltaR(gamma, rho)` inside
   `[min_delta_r_gamma_rho, max_delta_r_gamma_rho]`.

The rho is built with the charged pion mass hypothesis, regardless of the
stored `PFCand_mass`. The source mass is printed only as a diagnostic.

## Dataset-Scale Signal Production

Stage 11 added a manifest-driven production wrapper for the Run-3 signal sample:

```text
/GluGluHtoRhoG_Par-M-125_TuneCP5_13p6TeV_powheg-pythia8-evtgen/RunIII2024Summer24NanoAODv15-150X_mcRun3_2024_realistic_v2-v2/NANOAODSIM
```

The wrapper is intentionally script-level orchestration. It does not change the
candidate reconstruction, cuts, or branch mapping. It builds the Rust examples
once, runs the compiled HToRhoGamma executable once per selected file, merges
the candidate CSVs, optionally writes one combined candidate ROOT skim with the
compiled CSV-to-ROOT helper, and invokes the CSV plotting script.

Resolve the DAS dataset through the existing `nano-cli`/`nano-das` path:

```bash
python scripts/run_scouting_hrhogamma_signal.py \
  --dataset /GluGluHtoRhoG_Par-M-125_TuneCP5_13p6TeV_powheg-pythia8-evtgen/RunIII2024Summer24NanoAODv15-150X_mcRun3_2024_realistic_v2-v2/NANOAODSIM \
  --config configs/scouting/h_rho_gamma.toml \
  --outdir outputs/scouting_hrhogamma_signal \
  --resolve-das \
  --xrootd \
  --download-remote \
  --max-files 5 \
  --dry-run
```

Run a small manifest or local-file test:

```bash
python scripts/run_scouting_hrhogamma_signal.py \
  --manifest outputs/scouting_hrhogamma_signal/manifest.json \
  --config configs/scouting/h_rho_gamma.toml \
  --outdir outputs/scouting_hrhogamma_signal \
  --max-files 1
```

or:

```bash
python scripts/run_scouting_hrhogamma_signal.py \
  --local-files /path/to/file1.root /path/to/file2.root \
  --config configs/scouting/h_rho_gamma.toml \
  --outdir outputs/scouting_hrhogamma_signal \
  --max-events-per-file 100
```

The safe default is to process at most 5 files when `--max-files` is omitted.
Use `--all-files` only when intentionally running the full resolved sample:

```bash
python scripts/run_scouting_hrhogamma_signal.py \
  --manifest outputs/scouting_hrhogamma_signal/manifest.json \
  --config configs/scouting/h_rho_gamma.toml \
  --outdir outputs/scouting_hrhogamma_signal_full \
  --all-files
```

Native `root://` reading is not implemented in the current Rust ROOT reader.
For DAS/XRootD production, ask the script to cache remote files locally before
processing:

```bash
python scripts/run_scouting_hrhogamma_signal.py \
  --dataset /GluGluHtoRhoG_Par-M-125_TuneCP5_13p6TeV_powheg-pythia8-evtgen/RunIII2024Summer24NanoAODv15-150X_mcRun3_2024_realistic_v2-v2/NANOAODSIM \
  --config configs/scouting/h_rho_gamma.toml \
  --outdir outputs/scouting_hrhogamma_signal_cached \
  --resolve-das \
  --xrootd \
  --download-remote \
  --max-files 1 \
  --max-events-per-file 100
```

Remote inputs beginning with `root://` or `/store/` require
`--download-remote`. A `/store/...` LFN is converted to the global redirector
form `root://cms-xrd-global.cern.ch//store/...` for `xrdcp`. Cached files are
named deterministically from the file index, a short hash of the remote source,
and the source basename, so repeated runs can reuse non-empty cache files.

Useful switches:

- `--manifest path/to/manifest.json`: reuse an existing nano-das manifest.
- `--resolve-das`: call `cargo run -p nano-cli -- dataset resolve`.
- `--local-files file1.root file2.root`: run directly over local files.
- `--xrootd`: use manifest global XRootD URLs instead of local paths/LFNs.
- `--download-remote`: copy `root://` or `/store/` inputs into the local cache
  before running the Rust reader.
- `--cache-dir path`: choose the cache directory; defaults to
  `<outdir>/cache`.
- `--download-tool xrdcp`: choose the remote copy command.
- `--download-timeout SECONDS`: cap each remote copy attempt.
- `--force-download`: overwrite an existing non-empty cached file.
- `--clean-cache`: remove a cached ROOT file after successful processing.
- `--keep-cache`: explicit spelling of the default cache policy.
- `--release`: build and run `target/release/examples/*`.
- `--no-build`: skip compilation and require existing example binaries.
- `--use-cargo-run`: restore per-file `cargo run` behavior for debugging.
- `--max-events-per-file N`: cap each example invocation.
- `--skip-existing`: reuse existing per-file CSV/stdout outputs.
- `--plots-only`: merge and plot existing per-file CSVs.
- `--no-root`: skip combined ROOT skim writing.
- `--no-csv`: skip candidate CSV output and plotting.
- `--dry-run`: resolve/select files and write the production summary only.

The deterministic output layout is:

```text
outputs/scouting_hrhogamma_signal/
manifest.json
production_summary.txt
combined_candidates.csv
combined_candidates.root
combined_root.stdout.txt
combined_root.stderr.txt
plots.stdout.txt
plots.stderr.txt
build_scouting_h_rho_gamma.stdout.txt
build_scouting_h_rho_gamma.stderr.txt
build_scouting_h_rho_gamma_csv_to_root.stdout.txt
build_scouting_h_rho_gamma_csv_to_root.stderr.txt
cache/
file_000001_<hash>_<basename>.root
per_file/
file_000001.download.stdout.txt
file_000001.download.stderr.txt
file_000001.stdout.txt
file_000001.candidates.csv
file_000002.stdout.txt
file_000002.candidates.csv
plots/
summary.txt
h_mass.png
rho_mass.png
photon_pt.png
rho_pt.png
pi_plus_pt.png
pi_minus_pt.png
h_pt.png
delta_r_pipi.png
delta_r_gamma_rho.png
rho_pt_over_photon_pt.png
h_mass_vs_rho_mass.png
photon_pt_vs_h_mass.png
rho_pt_vs_h_mass.png
delta_r_gamma_rho_vs_h_mass.png
```

`production_summary.txt` records the dataset, manifest path, DAS resolver mode,
selected file count, execution binary, build mode, cache directory, download
tool, cache policy, per-file original and cached inputs, per-file download and
run statuses, per-file processed and accepted counts, combined CSV row count,
combined ROOT path, and plot status. The plotting summary records candidate
rows, unique events, duplicate event entries, min/mean/max values, approximate
`h_mass` and `rho_mass` quantiles, the broad
`100 < h_mass < 150` count, and the rho-window count from the config when the
config is readable.

## Known Limitations

- This runs on NanoAODv15-like signal MC with ordinary `Photon_*` and
  `PFCand_*` collections plus scouting trigger bits; it is not confirmed to be
  reduced HLT scouting object content.
- There is no truth matching or generator-level validation.
- There are no jet, L1, trigger-efficiency, isolation, or category studies.
- The ROOT output is a candidate skim, not a full event skim or analysis ntuple.
- The Python plots are signal-sample sanity plots over the candidate CSV, not a
  replacement for a final histogramming or statistical workflow.
- DAS access depends on the local `dasgoclient`/grid environment used by
  `nano-cli dataset resolve`.
- Native remote ROOT reading is not implemented in `nano-rootio`; DAS/XRootD
  production currently depends on `xrdcp`-style local caching with valid grid
  credentials and reachable redirectors.
- Track-quality cuts using `dz`, `dxy`, or object quality flags are deferred
  because those branches were not part of the confirmed local branch set.

## Next Extension Points

- Add Condor or workflow integration after the manifest-scale script is stable.
- Add analysis-grade histogram output after the CSV and candidate-skim checks
  are validated.
- Add truth matching and generator-level validation as a separate, explicit
  physics-validation stage.
- Extend the branch catalogue when true scouting-object files are available.
