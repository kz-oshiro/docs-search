"""Check that the shared corpus is reproducible and matches its case inventory."""

import hashlib
import json
import subprocess
import sys
import tempfile
import unittest
import xml.etree.ElementTree as ET
from pathlib import Path
from zipfile import ZipFile

HERE = Path(__file__).resolve().parent
GENERATOR = HERE / "generate-fixtures.py"


def generate(path):
    subprocess.run([sys.executable, str(GENERATOR), "--output", str(path)], check=True, capture_output=True)


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


class GeneratorTests(unittest.TestCase):
    def test_deterministic_corpus_and_case_references(self):
        with tempfile.TemporaryDirectory(prefix="doc-search-generator-") as temporary:
            first = Path(temporary) / "first"
            second = Path(temporary) / "second"
            generate(first)
            generate(second)
            left = {str(p.relative_to(first)).replace("\\", "/"): digest(p) for p in first.rglob("*") if p.is_file()}
            right = {str(p.relative_to(second)).replace("\\", "/"): digest(p) for p in second.rglob("*") if p.is_file()}
            self.assertEqual(left, right)

            manifest = json.loads((first / "manifest.json").read_text(encoding="utf-8"))
            cases = json.loads((first / "backend-cases.json").read_text(encoding="utf-8"))
            self.assertEqual(manifest["seed"], cases["seed"])
            self.assertEqual(len(manifest["files"]), 11)
            self.assertEqual(set(manifest["files"]), {name for name in left if name.startswith("search/")})
            self.assertEqual([p.suffix for p in (first / "search").rglob("*") if p.is_file()].count(".xlsx"), 4)

            for case in cases["cases"]:
                expected = case["expected"]
                if "counts" in expected:
                    self.assertEqual(expected["counts"]["resultCount"], len(expected["results"]), case["id"])
                    self.assertEqual(expected["counts"]["issueCount"], len(expected["issues"]), case["id"])
                for item in expected.get("results", []) + expected.get("issues", []):
                    self.assertTrue((first / "search" / item["file"]).is_file(), (case["id"], item["file"]))

            office = [p for p in (first / "search").rglob("*") if p.suffix in (".xlsx", ".xlsm", ".pptx", ".docx") and p.name != "broken.xlsx" and not p.name.startswith("~$")]
            self.assertEqual(len(office), 5)
            for path in office:
                with self.subTest(path=path.name), ZipFile(path) as archive:
                    self.assertIsNone(archive.testzip())
                    names = archive.namelist()
                    self.assertIn("[Content_Types].xml", names)
                    self.assertIn("_rels/.rels", names)
                    for name in names:
                        if name.endswith((".xml", ".rels", ".vml")):
                            ET.fromstring(archive.read(name))
            self.assertGreater((first / "search/spreadsheets/needle-book.xlsx").stat().st_size, 100_000)
            self.assertGreater((first / "search/nested/team-briefing.pptx").stat().st_size, 5_000)
            self.assertGreater((first / "search/documents/operations-guide.docx").stat().st_size, 5_000)
            self.assertGreater((first / "search/notes/research-log.txt").stat().st_size, 10_000)

    def test_load_profile_keeps_acceptance_corpus_separate(self):
        with tempfile.TemporaryDirectory(prefix="doc-search-load-") as temporary:
            output = Path(temporary) / "corpus"
            subprocess.run([sys.executable, str(GENERATOR), "--output", str(output), "--profile", "load"], check=True, capture_output=True)
            manifest = json.loads((output / "manifest.json").read_text(encoding="utf-8"))
            self.assertEqual(manifest["profile"], "load")
            self.assertEqual(len(manifest["files"]), 91)
            self.assertEqual(sum(name.startswith("search/") for name in manifest["files"]), 11)
            self.assertEqual(sum(name.startswith("load/") for name in manifest["files"]), 80)
            for suffix in (".xlsx", ".pptx", ".docx", ".txt"):
                self.assertEqual(sum(name.startswith("load/") and name.endswith(suffix) for name in manifest["files"]), 20)
            self.assertEqual(digest(output / "backend-cases.json"), digest(HERE / "backend-cases.json"))


if __name__ == "__main__":
    unittest.main()
