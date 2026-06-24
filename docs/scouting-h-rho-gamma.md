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

Optional CSV output writes every accepted H candidate:

```bash
cargo run -p nano-io --example scouting_h_rho_gamma -- /path/to/input.root 100 configs/scouting/h_rho_gamma.toml --csv candidates.csv
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

## Branch Mapping

The branch catalogue is in `configs/branches/scouting_run3.yaml`. The current
example uses explicit branch names matching that mapping:

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

The example currently keeps these values as local constants so that the demo is
self-contained fallback values. At runtime, it loads the same values from the
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

## Known Limitations

- This runs on NanoAODv15-like signal MC with ordinary `Photon_*` and
  `PFCand_*` collections plus scouting trigger bits; it is not confirmed to be
  reduced HLT scouting object content.
- There is no truth matching or generator-level validation.
- There are no jet, L1, trigger-efficiency, isolation, or category studies.
- There is no ROOT output, histogram output, workflow integration, DAS
  integration, or native xrootd reading in this demo.
- Track-quality cuts using `dz`, `dxy`, or object quality flags are deferred
  because those branches were not part of the confirmed local branch set.
- The example uses explicit branch names rather than loading the TOML/YAML
  mapping at runtime.

## Next Extension Points

- Promote the cut values from `configs/scouting/h_rho_gamma.toml` into runtime
  configuration for the example.
- Move candidate-building helpers into reusable library code if another stage
  needs tests around the physics objects.
- Add optional histograms or structured output after the print-only behavior is
  validated.
- Add truth matching and generator-level validation as a separate, explicit
  physics-validation stage.
- Extend the branch catalogue when true scouting-object files are available.
