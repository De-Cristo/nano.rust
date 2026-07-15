#!/usr/bin/env python3
"""Generic candidate-table plotting entrypoint.

Stage 17A keeps this as a compatibility wrapper around the current
HToRhoGamma-era plotting backend. The stable command name is generic; the
backend can be generalized behind it without changing user workflows.
"""

from __future__ import annotations

import sys
from pathlib import Path
import importlib.util


def main() -> int:
    backend = Path(__file__).resolve().with_name("plot_h_rho_gamma_csv.py")
    sys.argv[0] = str(Path(__file__).name)
    spec = importlib.util.spec_from_file_location("plot_h_rho_gamma_csv", backend)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"could not load plotting backend: {backend}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return int(module.main())


if __name__ == "__main__":
    raise SystemExit(main())
