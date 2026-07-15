# HToRhoGamma Offline NanoAOD Workflows

This page lists the small stable workflow surface for the HToRhoGamma offline
NanoAOD demo. It separates user-facing framework commands from analysis-specific
commands and exploratory diagnostics.

## Stable Framework-Facing Workflows

### 1. Inspect or read a sample

Use the branch mapping and `nano-cli`/`nano-io` examples to verify that the
offline NanoAOD ROOT file can be read. For the HToRhoGamma Run-3 NanoAODv15
fields, use:

```text
configs/branches/h_rho_gamma_nanov15.yaml
```

### 2. Plot a candidate table

Framework-facing wrapper:

```bash
python scripts/plot_candidate_table.py candidates.csv \
  --outdir plots/candidate_table
```

This wrapper currently delegates to the HToRhoGamma-era plotting backend. It is
the stable name for future generic candidate-table plotting.

### 3. Write a candidate report

Framework-facing wrapper:

```bash
python scripts/write_candidate_report.py \
  --csv candidates.csv \
  --outdir outputs/candidate_table
```

This wrapper currently delegates to the HToRhoGamma-era report backend. Future
work should extract generic report pieces behind this stable name.

### 4. Run the HToRhoGamma plugin/demo

The current analysis demonstrator remains:

```bash
python scripts/run_h_rho_gamma_signal.py \
  --local-dir /path/to/root/files \
  --config configs/h_rho_gamma.toml \
  --outdir outputs/h_rho_gamma_signal \
  --max-files 5
```

This is stable as a plugin/demo workflow, not as the framework identity.

### 5. Use explicit plot and category policy

The Stage 16C policy pattern is framework-relevant:

```bash
python scripts/analyze_h_rho_gamma_hgamma_closure_quality.py \
  --candidate-csv combined_candidates.csv \
  --outdir /tmp/h_rho_gamma_quality \
  --plot-config configs/h_rho_gamma_plotting.toml \
  --plot-profile physics_focus \
  --quality-config configs/h_rho_gamma_quality_categories.toml
```

The command itself is HToRhoGamma/dev-only, but the design rule is stable:
agents and humans should change explicit configs, not hidden plotting code.

## Generic Framework Example

The planned generic framework example is:

```text
scouting_photon_track_pair
```

It should be a minimal photon plus opposite-sign charged-object pair example
with generic column names such as `candidate_mass`, `pair_mass`,
`charged_plus_pt`, and `charged_minus_pt`. It is defined in
[`framework-plugin-boundary.md`](framework-plugin-boundary.md) and should be
implemented in a follow-up stage.

Until that example exists, use HToRhoGamma as the plugin/demo that exercises the
same pattern with analysis-specific names.

## Dev-Only Commands

These commands are useful for reproducing the development history, but they are
not recommended as the stable user workflow:

| Command | Classification | Why |
| --- | --- | --- |
| `scripts/survey_h_rho_gamma_genpart_topology.py` | dev-only | Signal-sample GenPart topology survey. |
| `scripts/diagnose_h_rho_gamma_pion_truth_proxy.py` | dev-only | Diagnosis of unusable pion-level GenPart truth. |
| `scripts/analyze_h_rho_gamma_hgamma_closure_quality.py` | dev-only | Hgamma closure-quality and threshold-scan study. |
| `crates/nano-io/examples/h_rho_gamma_genpart_survey.rs` | dev-only | Per-file backend for topology survey. |
| `crates/nano-io/examples/h_rho_gamma_pion_truth_diag.rs` | dev-only | Per-file backend for pion truth diagnostic. |

Dev-only does not mean disposable. It means the command is not part of the
stable framework promise and may move under `scripts/dev/` after compatibility
wrappers are in place.

## Compatibility Policy

Existing Stage 16 commands should keep working while the framework boundary is
introduced. Prefer this migration sequence:

1. Add stable generic names.
2. Keep old HToRhoGamma names as plugin/demo entrypoints.
3. Add deprecation or transition notes only after wrappers exist.
4. Move exploratory files only when tests cover the new paths.
