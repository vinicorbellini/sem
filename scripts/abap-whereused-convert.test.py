#!/usr/bin/env python3
"""Round-trip test of scripts/abap-whereused-convert.py.

Run: python3 scripts/abap-whereused-convert.test.py
"""

import importlib.util
import json
import sys
import unittest
from pathlib import Path

sys.dont_write_bytecode = True  # no scripts/__pycache__ from loading the converter

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("convert", ROOT / "scripts/abap-whereused-convert.py")
convert = importlib.util.module_from_spec(spec)
spec.loader.exec_module(convert)

GREP = ROOT / "crates/sem-core/tests/fixtures/abap/inside-sap/whereused.grep.json"
TASKS = ROOT / "bench/abap-agent/tasks/b1_whereused.json"


def caller_entities(rows):
    return sorted((r["object"].lower(), r["include"].lower(), r["file"], int(r["line"])) for r in rows)


class RoundTrip(unittest.TestCase):
    def test_grep_truth_round_trips(self):
        src = json.loads(GREP.read_text())
        tasks = json.loads(TASKS.read_text())
        harness = convert.to_harness(src, tasks, {}, {})
        self.assertEqual(set(harness["targets"]), {t["id"] for t in tasks["tasks"]})
        back = convert.from_harness(harness, tasks)
        for task in tasks["tasks"]:
            key = convert.key_of_target(task["target"])
            self.assertEqual(caller_entities(back[key]), caller_entities(src[key]), key)
        # and harness -> source -> harness is the identity
        again = convert.to_harness(back, tasks, {}, {})
        self.assertEqual(again["targets"], harness["targets"])

    def test_sapcli_rows_resolve_include(self):
        # A sapcli row has no file: the include names it, and a method include
        # goes through the method map.
        src = {"ZCL_A=>RUN": [
            {"object": "ZCL_B", "type": "CLAS", "include": "ZCL_B=========================CM002", "line": 4},
            {"object": "ZCL_B", "type": "CLAS", "include": "ZCL_B=========================CCAU", "line": 30},
            {"object": "ZFOO", "type": "PROG", "include": "ZFOO", "line": 7},
        ]}
        tasks = {"tasks": [{"id": "t1", "target": "zcl_a=>run"}]}
        harness = convert.to_harness(src, tasks, {"ZCL_B=========================CM002": "DO_IT"},
                                     {"zcl_b.clas.abap": "src/x"})
        self.assertEqual(harness["targets"]["t1"], [
            {"method": "zcl_b->do_it", "file": "src/x/zcl_b.clas.abap", "line": 4},
            {"method": "zcl_b->zcl_b=========================ccau", "file": "src/zcl_b.clas.testclasses.abap", "line": 30},
            {"method": "zfoo->zfoo", "file": "src/zfoo.prog.abap", "line": 7},
        ])

    def test_keys(self):
        self.assertEqual(convert.key_of_target("zcl_x->m"), "ZCL_X=>M")
        self.assertEqual(convert.key_of_target("zcl_x=>m"), "ZCL_X=>M")
        self.assertEqual(convert.key_of_target("zif_x~m"), "ZIF_X~M")


if __name__ == "__main__":
    unittest.main()
