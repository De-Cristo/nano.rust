# HToRhoGamma Dev-Only Scripts

This directory is reserved for exploratory diagnostics that helped develop the
HToRhoGamma offline NanoAOD demo but are not part of the stable framework
command surface.

Stage 17A does not move existing scripts here yet, because old Stage 16 commands
must remain reproducible. The intended future moves are:

```text
scripts/survey_h_rho_gamma_genpart_topology.py
scripts/diagnose_h_rho_gamma_pion_truth_proxy.py
scripts/analyze_h_rho_gamma_hgamma_closure_quality.py
```

Those scripts answer HToRhoGamma-specific questions about GenPart topology,
pion truth availability, and photon-anchored Higgs closure. They should remain
documented and testable, but they should not be advertised as the generic
NanoAOD framework workflow.

Promotion rule:

```text
If a dev-only script becomes generally useful, extract the generic behavior into
a stable wrapper or helper first, then keep analysis-specific diagnostics here.
```
