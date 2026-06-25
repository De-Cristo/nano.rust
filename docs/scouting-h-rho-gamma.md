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
compiled CSV-to-ROOT helper, invokes the CSV plotting script, and writes a
human-readable `physics_summary.md` report.

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

Run every ROOT file in a local signal directory with optional GenPart truth
truth-proxy validation:

```bash
MPLCONFIGDIR=/tmp/matplotlib-cache-stage15 .venv/bin/python scripts/run_scouting_hrhogamma_signal.py \
  --local-dir /home/lzhang/lxplus/scouting/nano_data/GluGluHtoRhoG_Par-M-125 \
  --config configs/scouting/h_rho_gamma.toml \
  --outdir /tmp/scouting_hrhogamma_signal_stage15_full_local \
  --all-files \
  --truth
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
- `--local-dir path`: discover local files in sorted order from a directory.
- `--local-glob "*.root"`: choose the pattern used by `--local-dir`.
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
- `--report-only`: reuse existing CSV outputs and regenerate the plot summary
  plus `physics_summary.md` without rerunning ROOT processing.
- `--report-title "..."`: set the Markdown report title.
- `--no-physics-report`: skip `physics_summary.md` generation.
- `--truth`: request optional GenPart truth validation and truth-augmented
  candidate output.
- `--no-root`: skip combined ROOT skim writing.
- `--no-csv`: skip candidate CSV output and plotting.
- `--dry-run`: resolve/select files and write the production summary only.

The deterministic output layout is:

```text
outputs/scouting_hrhogamma_signal/
manifest.json
production_summary.txt
physics_summary.md
combined_candidates.csv
combined_candidates.root
combined_root.stdout.txt
combined_root.stderr.txt
plots.stdout.txt
plots.stderr.txt
physics_report.stdout.txt
physics_report.stderr.txt
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
truth_proxy_match_thresholds.png
truth_proxy_match_thresholds.pdf
h_mass_truth_proxy_matched_vs_unmatched.png
h_mass_truth_proxy_matched_vs_unmatched.pdf
rho_mass_truth_proxy_matched_vs_unmatched.png
rho_mass_truth_proxy_matched_vs_unmatched.pdf
reco_h_mass_minus_gen_h_proxy_mass.png
reco_h_mass_minus_gen_h_proxy_mass.pdf
reco_rho_mass_minus_gen_rho_proxy_mass.png
reco_rho_mass_minus_gen_rho_proxy_mass.pdf
delta_r_reco_photon_gen_photon.png
delta_r_reco_photon_gen_photon.pdf
reco_photon_pt_over_gen_photon_pt.png
reco_photon_pt_over_gen_photon_pt.pdf
reco_h_mass_vs_gen_h_proxy_mass.png
reco_h_mass_vs_gen_h_proxy_mass.pdf
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

`physics_summary.md` is written automatically when
`combined_candidates.csv` exists. It records dataset/config provenance, selected
and successful file counts, total processed events and accepted candidates,
candidate rate per processed event, candidate-variable summaries for Higgs,
rho, photon, pion, and angular observables, broad Higgs-window counts, the
configured rho-window count, a plot index, missing-plot diagnostics, cautious
interpretation notes, and current limitations. If `matplotlib` is unavailable,
the report is still written and the plot section states that PNG generation was
skipped.

When `--truth` is enabled, the Rust example attempts to read standard NanoAOD
GenPart branches: `nGenPart`, `GenPart_pdgId`,
`GenPart_genPartIdxMother`, `GenPart_status`, `GenPart_statusFlags`,
`GenPart_pt`, `GenPart_eta`, `GenPart_phi`, and `GenPart_mass`. Stage 15 uses a
topology-aware truth-proxy strategy by default. This reflects the Stage 14B
survey result: the local signal sample has a reliable Higgs-descendant photon
anchor, but the GenPart record does not provide a useful
`rho0 -> pi+ pi-` ancestry chain for the charged pions.

The truth-proxy matcher:

1. Selects the closest GenPart photon with `pdgId == 22` and Higgs ancestry.
2. Selects the nearest plausible final-state GenPart `pi+` and `pi-` by
   `DeltaR`, without requiring Higgs or rho ancestry for the pions.
3. Builds a generator rho proxy from the matched pions and a generator H proxy
   from the matched photon plus pions.
4. Defines the primary `truth_proxy_matched` flag as all three object matches
   satisfying `DeltaR < 0.1`.
5. Also records looser diagnostic flags for `DeltaR < 0.2` and `DeltaR < 0.3`.

Truth mode appends explicit proxy-named CSV columns including
`truth_strategy`, `truth_available`, `truth_proxy_matched`,
`truth_proxy_matched_dr_0p1`, `truth_proxy_matched_dr_0p2`,
`truth_proxy_matched_dr_0p3`, `truth_photon_anchor_available`,
`nearest_gen_pi_plus_available`, `nearest_gen_pi_minus_available`,
GenPart photon and pion kinematics, `gen_rho_proxy_*`, `gen_h_proxy_*`,
reco-gen `DeltaR` values, and response variables such as
`reco_h_mass_minus_gen_h_proxy_mass` and
`reco_h_pt_over_gen_h_proxy_pt`. The combined ROOT skim stores the same
candidate-level quantities and encodes `truth_strategy` as
`truth_strategy_code`: `0=none`, `1=explicit_chain`, `2=topology_proxy`.

The production report adds `truth_proxy_summary.md` when these columns are
present. It records candidate-level match fractions, event-level
efficiency-like quantities, response and resolution summaries, and links to the
truth-proxy plots. These are demonstrator-level checks over accepted
candidates; they are not final analysis efficiencies.

Plots use a compact HEP-style matplotlib configuration when plotting is
available: 6-inch-scale figures, 10-12 pt fonts, step histograms, axis units,
tight/constrained layout, optional CMS-style labels through `mplhep`, and both
PNG and PDF output. If matplotlib is unavailable, summaries and reports are
still written.

## GenPart Topology Survey

Stage 14B adds a diagnostic survey for understanding how the signal MC actually
stores generator particles before changing the truth finder. It does not change
candidate reconstruction, cuts, CSV production, ROOT writing, or plotting.

Run a small local survey:

```bash
python scripts/survey_hrhogamma_genpart_topology.py \
  --local-dir /home/lzhang/lxplus/scouting/nano_data/GluGluHtoRhoG_Par-M-125 \
  --outdir /tmp/scouting_hrhogamma_genpart_survey_small \
  --max-files 2 \
  --max-events-per-file 1000
```

Run the full local signal directory:

```bash
python scripts/survey_hrhogamma_genpart_topology.py \
  --local-dir /home/lzhang/lxplus/scouting/nano_data/GluGluHtoRhoG_Par-M-125 \
  --outdir /tmp/scouting_hrhogamma_genpart_survey_full \
  --all-files
```

The runner builds and calls:

```bash
target/debug/examples/scouting_h_rho_gamma_genpart_survey \
  input.root [max-events] \
  --out-json per_file/file_000001.summary.json \
  --out-text per_file/file_000001.summary.txt \
  --out-examples per_file/file_000001.examples.txt
```

Useful switches:

- `--local-glob "*.root"`: choose the local file pattern.
- `--max-files N`: survey the first `N` sorted files.
- `--all-files`: survey every matched file.
- `--max-events-per-file N`: cap each per-file survey.
- `--examples N`: keep at most `N` example events per category.
- `--release`: run the release example binary.
- `--no-build`: require an existing example binary.
- `--skip-existing`: reuse existing per-file JSON/stdout outputs.
- `--dry-run`: write the selected-file plan without reading ROOT files.

The output layout is:

```text
/tmp/scouting_hrhogamma_genpart_survey_full/
genpart_topology_summary.txt
genpart_topology_summary.json
per_file/
file_000001.summary.json
file_000001.summary.txt
file_000001.examples.txt
file_000001.stdout.txt
```

The survey inspects standard NanoAOD GenPart branches:
`nGenPart`, `GenPart_pdgId`, `GenPart_genPartIdxMother`, `GenPart_status`,
`GenPart_statusFlags`, `GenPart_pt`, `GenPart_eta`, `GenPart_phi`, and
`GenPart_mass`. It reports file/event counts, signed and absolute pdgId
frequency tables, status and statusFlags tables for key particles, mother to
daughter relation counts, ancestry checks, and bounded examples for events with
Higgs, rho0, gamma plus pions without rho0, and events where the simple Stage
14 truth finder would say `not_found`.

The small two-file survey over 1000 events per file found `25` and `22` in all
events, a small number of `113` entries, and some charged pions, but no explicit
`113 -> pi+ pi-` relation and no Higgs/rho ancestry for the charged pions. It
therefore recommends treating the current ancestry as insufficient. Stage 15
implements that recommendation as a topology-aware truth-proxy matcher using
the Higgs-descendant photon plus nearest final-state `pi+ pi-` proxies.

## Known Limitations

- This runs on NanoAODv15-like signal MC with ordinary `Photon_*` and
  `PFCand_*` collections plus scouting trigger bits; it is not confirmed to be
  reduced HLT scouting object content.
- Truth matching is a preliminary GenPart sanity check, not an efficiency or
  resolution model.
- The GenPart topology survey is diagnostic; it does not yet implement the
  Stage 15 truth-matching strategy recommended by the survey output.
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
- Truth-proxy matching uses nearest GenPart charged pions because the local
  signal sample does not expose useful pion ancestry for this purpose. The
  resulting proxy response plots are validation diagnostics, not a substitute
  for a generator-level decay-chain truth definition.

## Next Extension Points

- Add Condor or workflow integration after the manifest-scale script is stable.
- Add analysis-grade histogram output after the CSV and candidate-skim checks
  are validated.
- Refine the truth-proxy matcher after comparing it with any future sample that
  exposes an explicit `rho0 -> pi+ pi-` generator chain.
- Extend the branch catalogue when true scouting-object files are available.
