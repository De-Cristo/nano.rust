#!/usr/bin/env python3
"""Generic candidate-report entrypoint.

Stage 17A keeps this as a compatibility wrapper around the current
HToRhoGamma-era report backend. The stable command name is generic; the backend
can be generalized behind it without changing user workflows.
"""

from __future__ import annotations

import sys
from pathlib import Path
import importlib.util


def main() -> int:
    backend = Path(__file__).resolve().with_name("write_h_rho_gamma_report.py")
    sys.argv[0] = str(Path(__file__).name)
    spec = importlib.util.spec_from_file_location("write_h_rho_gamma_report", backend)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"could not load report backend: {backend}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return int(module.main())


if __name__ == "__main__":
    raise SystemExit(main())
