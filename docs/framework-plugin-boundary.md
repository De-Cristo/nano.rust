# Framework/Plugin Boundary

This note defines the offline-NanoAOD-facing boundary for `nano.rust`. It is a
software-structure document, not a physics prescription.

The public model should be:

```text
nano.rust framework core
  reusable NanoAOD I/O, branch catalogues, execution, candidate-table
  plotting/reporting, and workflow helpers

universal scouting example
  a minimal photon + opposite-sign charged-object pair demonstrator that shows
  how to use the framework without embedding one analysis identity

analysis plugin / demonstrator
  HToRhoGamma-specific reconstruction, configs, truth closure, and reports

exploratory diagnostics
  one-off studies used to understand a sample or validate a proposed analysis
  handle; useful, but not part of the stable framework surface
```

The goal is that a new reader sees `nano.rust` as a reusable NanoAOD
framework. HToRhoGamma remains valuable, but it is one plugin-like example that
consumes the framework.

## Layer Definitions

| Layer | Owns | Does Not Own |
| --- | --- | --- |
| Core framework | ROOT/NanoAOD reading and writing, branch catalogues, sample/manifest execution, generic candidate table handling, generic plotting/reporting policy | Higgs/rho labels, truth strategy choices, analysis-specific category definitions |
| Universal scouting example | Minimal photon + opposite-sign charged-object pair candidate, generic table columns, CSV/ROOT output, plot policy use | Rho mass target, pion-mass interpretation as an analysis claim, Higgs closure, GenPart recoil closure |
| Analysis plugin/demo | HToRhoGamma naming, cuts, config, truth closure, production runner, candidate-quality categories | Core I/O semantics, generic candidate table API, generic plot/report shell |
| Exploratory diagnostics | GenPart topology surveys, pion truth diagnosis, hgamma closure quality scans, one-off threshold grids | Stable framework command surface |

## Current File Classification

| File | Classification | Notes |
| --- | --- | --- |
| `crates/nano-rootio/src/*` | core framework | Owned ROOT reader/writer layer. |
| `crates/nano-io/src/samples.rs` | core framework | Sample normalization table. |
| `crates/nano-io/src/datacard.rs` | core framework | Statistical output handoff, not scouting-specific. |
| `configs/branches/h_rho_gamma_nanov15.yaml` | HToRhoGamma plugin/demo | Analysis-specific Run-3 offline NanoAODv15 branch mapping. |
| `scripts/plot_candidate_table.py` | core framework wrapper | Generic wrapper over the current candidate-table plotting backend. |
| `scripts/write_candidate_report.py` | core framework wrapper | Generic wrapper over the current report backend. |
| `scripts/plot_h_rho_gamma_csv.py` | HToRhoGamma plugin/demo, candidate for generalization | Contains generic candidate-table behavior plus HToRhoGamma labels and truth-column handling. |
| `scripts/write_h_rho_gamma_report.py` | HToRhoGamma plugin/demo, candidate for generalization | Stable for HToRhoGamma production reports; generic report pieces should be extracted later. |
| `scripts/h_rho_gamma_plotting_policy.py` | HToRhoGamma plugin/demo, candidate for generalization | Plot-profile/category machinery is generic in shape, but the module name and defaults are HToRhoGamma-specific. |
| `crates/nano-io/src/h_rho_gamma.rs` | HToRhoGamma plugin/demo | Analysis-specific candidate builder and truth helpers. |
| `crates/nano-io/examples/h_rho_gamma.rs` | HToRhoGamma plugin/demo | Analysis-specific CLI example. |
| `crates/nano-io/examples/h_rho_gamma_csv_to_root.rs` | HToRhoGamma plugin/demo | Candidate skim writer for the current demo output schema. |
| `scripts/run_h_rho_gamma_signal.py` | HToRhoGamma plugin/demo | Stable analysis-demo production runner; may later call generic workflow helpers. |
| `scripts/demo_h_rho_gamma.sh` | HToRhoGamma plugin/demo | Local single-file demo wrapper. |
| `configs/h_rho_gamma.toml` | HToRhoGamma plugin/demo | Analysis cuts and constants. |
| `configs/h_rho_gamma_plotting.toml` | HToRhoGamma plugin/demo, policy example | Demonstrates explicit plot profiles. |
| `configs/h_rho_gamma_quality_categories.toml` | HToRhoGamma plugin/demo | Analysis-specific diagnostic categories. |
| `scripts/survey_h_rho_gamma_genpart_topology.py` | exploratory/dev-only | Sample-discovery diagnostic. |
| `scripts/diagnose_h_rho_gamma_pion_truth_proxy.py` | exploratory/dev-only | Pion-truth diagnostic; not stable framework surface. |
| `scripts/analyze_h_rho_gamma_hgamma_closure_quality.py` | exploratory/dev-only | Closure-quality and threshold-scan diagnostic. |
| `crates/nano-io/src/genpart_survey.rs` | exploratory/dev-only | Support for HToRhoGamma GenPart topology survey. |
| `crates/nano-io/src/pion_truth_diagnosis.rs` | exploratory/dev-only | Support for pion truth diagnosis. |
| `crates/nano-io/examples/h_rho_gamma_genpart_survey.rs` | exploratory/dev-only | Per-file topology survey entrypoint. |
| `crates/nano-io/examples/h_rho_gamma_pion_truth_diag.rs` | exploratory/dev-only | Per-file pion truth diagnostic entrypoint. |
| `outputs/` | legacy/archive candidate | Generated production output, not source. |

## Universal Scouting Example

The framework-facing example should be named:

```text
scouting_photon_track_pair
```

It should demonstrate the reusable shape:

```text
input NanoAOD file or manifest
branch catalogue
photon candidate
opposite-sign charged PFCand/track pair
candidate four-vector
candidate table CSV/ROOT
plot policy
summary report
```

Generic output names should avoid HToRhoGamma semantics:

```text
candidate_mass
candidate_pt
pair_mass
pair_pt
charged_plus_pt
charged_minus_pt
delta_r_pair
delta_r_photon_pair
```

The universal example should not contain:

```text
rho_mass
h_mass
rho_mass_target
Higgs closure
gen rho recoil
HToRhoGamma truth labels
```

HToRhoGamma can keep analysis-specific aliases and interpretations on top of
the same lower-level pattern.

## Promotion Rules

Use these rules when deciding where new code belongs:

1. If a tool is generic and reusable, move or generalize it into framework helper
   code or a generic script name.
2. If a tool is HToRhoGamma-specific but stable, keep it under analysis/demo
   naming and document the plugin boundary.
3. If a tool is exploratory and no longer part of the stable path, document it as
   dev-only and move it under `scripts/dev/` only when tests/imports can be
   updated safely.
4. If a scan parameter was useful only once, keep the conclusion in docs and keep
   the operational scan out of the stable framework workflow.
5. Stable commands should apply configs. They should not hide physics choices in
   script internals.

## Transition Structure

Target structure for a later stage:

```text
configs/
  branches/
    h_rho_gamma_nanov15.yaml
  examples/
    scouting_photon_track_pair.toml
    scouting_photon_track_pair_plotting.toml
  analyses/
    h_rho_gamma.toml
    h_rho_gamma_plotting.toml
    h_rho_gamma_quality_categories.toml

scripts/
  run_scouting_example.py
  plot_candidate_table.py
  write_candidate_report.py
  dev/
    survey_h_rho_gamma_genpart_topology.py
    diagnose_h_rho_gamma_pion_truth_proxy.py
    analyze_h_rho_gamma_hgamma_closure_quality.py
```

Stage 17A intentionally keeps old HToRhoGamma paths alive. A later stage can add
new config paths and wrappers first, then move files once compatibility tests
cover both the old and new names.
