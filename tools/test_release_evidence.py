import unittest
from subprocess import CalledProcessError
from unittest import mock

from release_evidence import EDITION, PEERS, claims, peer_results


class EvidenceTest(unittest.TestCase):
    def setUp(self):
        self.vector = "tests/inventory.rs::switch_rx"
        self.passed = {"switch_rx", "edition_and_support_are_traceable"}
        self.row = {
            "edition": EDITION,
            "kind": "dpt",
            "id": "1.001",
            "section": "3.1",
            "encode": "unsupported",
            "encode_evidence": "",
            "decode": "tested",
            "decode_evidence": self.vector,
        }

    def test_reports_only_the_passing_direction(self):
        (result,) = claims([self.row], self.passed)
        self.assertEqual(
            (result["dpt"], result["direction"], result["result"], result["case"]),
            ("1.001", "decode", "vector-tested", self.vector),
        )

    def test_parameter_validation_is_separate_from_dpt_evidence(self):
        parameter = {
            **self.row,
            "kind": "parameter",
            "id": "1101h",
            "parameter_validation": "value",
            "encode": "not-applicable",
            "decode": "not-applicable",
            "decode_evidence": "",
        }
        self.assertEqual(
            claims([self.row, parameter], self.passed), claims([self.row], self.passed)
        )
        for patch in (
            {"encode": "tested"},
            {"kind": "dpt"},
            {"parameter_validation": "certified"},
            {"decode_evidence": self.vector},
            {"edition": ""},
        ):
            with self.subTest(patch=patch), self.assertRaises(ValueError):
                claims([self.row, {**parameter, **patch}], self.passed)

    def test_layout_evidence_is_not_promoted_to_a_full_codec(self):
        row = {**self.row, "id": "277.1200", "decode": "layout-tested"}
        (result,) = claims([row], self.passed)
        self.assertEqual(result["result"], "layout-tested")

    def test_peer_claims_require_a_successful_exchange(self):
        with mock.patch("release_evidence.subprocess.run") as invoke:
            skipped = peer_results(False)
            invoke.assert_not_called()
            self.assertEqual(set(skipped), set(PEERS))
            self.assertTrue(
                all(peer["result"] == "not-run" for peer in skipped.values())
            )
            invoke.side_effect = CalledProcessError(1, ["peer"])
            with self.assertRaises(CalledProcessError):
                peer_results(True)

    def test_unverified_or_reused_evidence_is_rejected(self):
        for patch in (
            {"decode_evidence": ""},
            {"edition": ""},
            {"decode": "certified"},
            {"kind": "parameter"},
            {"decode_evidence": "tests/other.rs::switch_rx"},
            {"encode": "tested", "encode_evidence": self.vector},
        ):
            with self.subTest(patch=patch), self.assertRaises(ValueError):
                claims([{**self.row, **patch}], self.passed)
        for missing in self.passed:
            with self.subTest(missing=missing), self.assertRaises(ValueError):
                claims([self.row], self.passed - {missing})


if __name__ == "__main__":
    unittest.main()
