#!/usr/bin/env python3

from __future__ import annotations

import importlib.util
import json
from pathlib import Path
import tempfile
import unittest


SCRIPT = Path(__file__).with_name("generate_3d_performance_artifact.py")
SPEC = importlib.util.spec_from_file_location("generate_3d_performance_artifact", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
generator = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(generator)


class PerformanceArtifactTests(unittest.TestCase):
    def write_case(
        self,
        criterion: Path,
        directory: str,
        group: str,
        function: str,
        value: str | None,
        elements: int,
    ) -> None:
        case = criterion / directory / "new"
        case.mkdir(parents=True)
        (case / "benchmark.json").write_text(
            json.dumps(
                {
                    "group_id": group,
                    "function_id": function,
                    "value_str": value,
                    "throughput": {"Elements": elements},
                    "full_id": f"{group}/{function}"
                    + (f"/{value}" if value else ""),
                }
            ),
            encoding="utf-8",
        )
        estimate = {
            "confidence_interval": {
                "confidence_level": 0.95,
                "lower_bound": 18.0,
                "upper_bound": 22.0,
            },
            "point_estimate": 20.0,
            "standard_error": 1.0,
        }
        (case / "estimates.json").write_text(
            json.dumps({"mean": estimate, "median": estimate}),
            encoding="utf-8",
        )
        (case / "sample.json").write_text(
            json.dumps(
                {
                    "sampling_mode": "Linear",
                    "iters": [1.0, 2.0, 3.0],
                    "times": [10.0, 40.0, 90.0],
                }
            ),
            encoding="utf-8",
        )

    def write_manifest(self, path: Path) -> None:
        path.write_text(
            json.dumps(
                {
                    "schema_version": 1,
                    "hash": "test hash contract",
                    "datasets": [
                        {
                            "id": "scatter-wave-100000",
                            "kind": "scatter",
                            "elements": 100000,
                            "hash": "fnv1a64:test",
                        },
                        {
                            "id": "surface-sinc-100",
                            "kind": "surface",
                            "rows": 100,
                            "columns": 100,
                            "hash": "fnv1a64:surface",
                        },
                    ],
                }
            ),
            encoding="utf-8",
        )

    def test_classifies_cold_warm_and_update_profiles(self) -> None:
        cases = [
            (
                {
                    "group_id": "3d/cpu/scatter",
                    "function_id": "compile",
                    "value_str": "100000",
                },
                "cpu.cold.scene_compile",
            ),
            (
                {
                    "group_id": "3d/cpu/retained-warm-frame",
                    "function_id": "surface-100x100-640x480",
                    "value_str": None,
                },
                "cpu.warm.unchanged_frame",
            ),
            (
                {
                    "group_id": "3d/gpu/retained-camera-no-readback",
                    "function_id": "scatter-100000-640x480",
                    "value_str": None,
                },
                "gpu.update.camera",
            ),
        ]
        for metadata, expected in cases:
            with self.subTest(expected=expected):
                self.assertEqual(
                    generator.classify_benchmark(metadata)["profile_id"], expected
                )

    def test_builds_sample_derived_artifact_without_gate_claims(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            criterion = root / "criterion"
            manifest = root / "manifest.json"
            self.write_manifest(manifest)
            self.write_case(
                criterion,
                "3d_cpu_retained-warm-frame/scatter-100000-640x480",
                "3d/cpu/retained-warm-frame",
                "scatter-100000-640x480",
                None,
                100000,
            )

            artifact = generator.build_artifact(criterion, manifest, None)

        self.assertEqual(artifact["$schema"], generator.SCHEMA_ID)
        self.assertEqual(artifact["coverage"]["measurement_count"], 1)
        measurement = artifact["measurements"][0]
        self.assertEqual(measurement["profile_id"], "cpu.warm.unchanged_frame")
        self.assertEqual(measurement["dataset"]["identity_status"], "verified")
        self.assertEqual(measurement["samples"]["per_iteration_ns"], [10.0, 20.0, 30.0])
        self.assertEqual(measurement["samples"]["median_ns"], 20.0)
        self.assertAlmostEqual(measurement["samples"]["p95_ns"], 29.0)
        self.assertEqual(measurement["diagnostics"]["status"], "unmeasured")
        self.assertTrue(
            all(gate["status"] == "not_evaluated" for gate in artifact["gates"])
        )

    def test_schema_tracks_generator_contract(self) -> None:
        schema = json.loads(
            Path(__file__)
            .with_name("ruviz_3d_performance.schema.json")
            .read_text(encoding="utf-8")
        )
        self.assertEqual(schema["$id"], generator.SCHEMA_ID)
        self.assertEqual(schema["properties"]["schema_version"]["const"], 1)
        profile_status = schema["$defs"]["profile"]["properties"]["status"]["enum"]
        self.assertEqual(profile_status, ["measured", "unmeasured"])
        self.assertEqual(
            schema["$defs"]["gate"]["properties"]["status"]["const"],
            "not_evaluated",
        )

    def test_committed_local_artifact_preserves_non_claiming_status(self) -> None:
        artifact = json.loads(
            (
                Path(__file__).resolve().parents[1]
                / "docs"
                / "benchmarks"
                / "ruviz-3d-performance-local-2026-07-24.json"
            ).read_text(encoding="utf-8")
        )
        schema = json.loads(
            Path(__file__)
            .with_name("ruviz_3d_performance.schema.json")
            .read_text(encoding="utf-8")
        )
        self.assertEqual(artifact["$schema"], schema["$id"])
        self.assertEqual(
            artifact["schema_version"],
            schema["properties"]["schema_version"]["const"],
        )
        self.assertEqual(artifact["collection_errors"], [])
        self.assertEqual(artifact["coverage"]["measurement_count"], 8)
        self.assertTrue(
            all(gate["status"] == "not_evaluated" for gate in artifact["gates"])
        )
        self.assertTrue(
            all(
                measurement["diagnostics"]["status"] == "unmeasured"
                for measurement in artifact["measurements"]
            )
        )


if __name__ == "__main__":
    unittest.main()
