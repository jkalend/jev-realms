from __future__ import annotations

import json
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import pipeline  # type: ignore
from glb_writer import write_glb  # type: ignore


class TinyMesh:
    vertices = [(-0.5, 0.0, -0.5), (0.5, 0.0, -0.5), (0.5, 1.0, -0.5), (-0.5, 1.0, -0.5)]
    faces = [(0, 1, 2, 3)]
    materials = ["stone"]

try:
    from PIL import Image
except ImportError:  # pragma: no cover - the test skips cleanly without Pillow
    Image = None  # type: ignore[assignment]


@unittest.skipUnless(Image is not None, "Pillow is required for bake tests")
class PipelineContractTests(unittest.TestCase):
    directions = [
        "front", "front_right", "right", "back_right",
        "back", "back_left", "left", "front_left",
    ]

    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        source = self.root / "source" / "Commoner.glb"
        source.parent.mkdir(parents=True)
        write_glb(TinyMesh(), source, "Commoner", {"stone": (128, 128, 128)})
        for animation in ("idle", "walk", "attack", "hit"):
            for direction in self.directions:
                path = self.root / "render" / "Commoner" / animation / direction / "000.png"
                path.parent.mkdir(parents=True, exist_ok=True)
                Image.new("RGBA", (32, 40), (180, 60, 40, 255)).save(path)
        self.manifest = {
            "version": 1,
            "assets": [{
                "id": "Commoner",
                "kind": "actor",
                "source": "source/Commoner.glb",
                "render_root": "render/Commoner",
                "cell_size": {"width": 32, "height": 40},
                "directions": self.directions,
                "animations": {"idle": 1, "walk": 1, "attack": 1, "hit": 1},
                "uvs": True,
                "materials": True,
                "rig": True,
                "pivot": "feet_center",
            }],
        }

    def tearDown(self) -> None:
        self.temp.cleanup()

    def test_validates_source_and_directional_renders(self) -> None:
        report = pipeline.validate_manifest(self.manifest, self.root)
        self.assertEqual(report["assets"], 1)
        self.assertEqual(report["render_files"], 32)

    def test_action_profile_tracks_weapon_and_boss_clips(self) -> None:
        self.manifest["assets"][0]["action_profile"] = {
            "id": "npc_staff",
            "weapon": "staff",
            "default_action": "staff_idle",
            "clips": {
                "idle": ["staff_idle"],
                "walk": ["staff_walk"],
                "attack": ["staff_cast"],
                "hit": ["staff_hit"],
            },
        }
        report = pipeline.validate_manifest(self.manifest, self.root, check_files=False)
        self.assertEqual(report["assets"], 1)
        self.manifest["assets"][0]["action_profile"]["clips"].pop("attack")
        with self.assertRaisesRegex(pipeline.PipelineError, "clips must include"):
            pipeline.validate_manifest(self.manifest, self.root, check_files=False)

    def test_default_action_must_be_a_declared_clip(self) -> None:
        self.manifest["assets"][0]["action_profile"] = {
            "id": "npc_staff",
            "weapon": "staff",
            "default_action": "missing_idle",
            "clips": {
                "idle": ["staff_idle"],
                "walk": ["staff_walk"],
                "attack": ["staff_cast"],
                "hit": ["staff_hit"],
            },
        }
        with self.assertRaisesRegex(pipeline.PipelineError, "default_action must name"):
            pipeline.validate_manifest(self.manifest, self.root, check_files=False)

    def test_declared_action_clips_must_exist_in_glb(self) -> None:
        with self.assertRaisesRegex(pipeline.PipelineError, "missing declared action clips"):
            pipeline.validate_glb(self.root / "source" / "Commoner.glb", {"missing_clip"})

    def test_bakes_deterministic_fragment(self) -> None:
        with tempfile.TemporaryDirectory() as output:
            result = pipeline.bake_manifest(
                self.manifest,
                self.root,
                Path(output),
                "actors-test",
            )
            self.assertEqual(result["cells"], 32)
            sheet = Path(result["sheet"])
            fragment = json.loads(Path(result["fragment"]).read_text(encoding="utf-8"))
            self.assertEqual(fragment["sheet"]["mapping"]["Commoner"], [0, 0])
            self.assertEqual(fragment["sheet"]["cell_size"], {"width": 32, "height": 40})
            with Image.open(sheet) as image:
                self.assertEqual(image.size, (8 * 32, 4 * 40))

    def test_missing_source_is_actionable(self) -> None:
        (self.root / "source" / "Commoner.glb").unlink()
        with self.assertRaisesRegex(pipeline.PipelineError, "missing GLB source"):
            pipeline.validate_manifest(self.manifest, self.root)

    def test_missing_uv_contract_is_rejected(self) -> None:
        self.manifest["assets"][0]["uvs"] = False
        with self.assertRaisesRegex(pipeline.PipelineError, "uvs must be true"):
            pipeline.validate_manifest(self.manifest, self.root, check_files=False)


if __name__ == "__main__":
    unittest.main()
