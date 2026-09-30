"""Exercise flash decisions with simulated I/O; never access a keyboard."""

import json
import subprocess
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
LIBRARY = ROOT / "lib/moergo-safe-flash.sh"


class SafeFlashTests(unittest.TestCase):
    def shell(self, script, *arguments):
        return subprocess.run(
            [
                "bash",
                "-c",
                'source "$1"; shift\n' + script,
                "test",
                str(LIBRARY),
                *arguments,
            ],
            text=True,
            capture_output=True,
            timeout=5,
            check=False,
        )

    def verdict(self, observations):
        result = self.shell(
            """
watch=8
SECONDS=0
index=0
samples=("$@")
target_present() {
  local sample=${samples[index]:-${samples[-1]}}
  index=$((index + 1))
  [ "$sample" = 1 ]
}
sleep() { SECONDS=$((SECONDS + 1)); }
verdict
""",
            *map(str, observations),
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        return result.stdout.strip()

    def test_stable_boot_including_delayed_enumeration(self):
        self.assertEqual(self.verdict([1]), "stable")
        self.assertEqual(self.verdict([0, 0, 1]), "stable")

    def test_disconnect_at_end_is_not_stable(self):
        self.assertEqual(self.verdict([1, 1, 0]), "crashloop")

    def test_absent_and_flapping_applications_fail(self):
        self.assertEqual(self.verdict([0]), "crashloop")
        self.assertEqual(self.verdict([1, 0, 1, 0, 1, 0, 1]), "crashloop")

    def test_health_checks_board_identity_and_peripheral_connection(self):
        for board, peripheral, connected, expected in [
            ("Glove80", "", False, 0),
            ("Go60", "", True, 1),
            ("Glove80", "1", False, 1),
            ("Glove80", "1", True, 0),
        ]:
            report = json.dumps(
                {
                    "name": board,
                    "batteries": [
                        {"name": "Central"},
                        {"name": "Peripheral 0", "connected": connected},
                    ],
                }
            )
            result = self.shell(
                """
board_name=Glove80
peripheral=$1
mock_report=$2
CONTROL=unused
timeout() { echo "$mock_report"; }
target_present
""",
                peripheral,
                report,
            )
            self.assertEqual(result.returncode, expected, result.stderr)

    def test_bad_arguments_fail_before_build_or_hardware_access(self):
        for board in ["glove80", "go60"]:
            for arguments in [
                ["--recover"],
                ["--watch-seconds"],
                ["/dev/null"],
                [str(LIBRARY), "--watch-seconds", "-1"],
                [str(LIBRARY), "--watch-seconds", "wat"],
                [str(LIBRARY), str(LIBRARY)],
            ]:
                result = subprocess.run(
                    [str(ROOT / "bin" / f"{board}-safe-flash"), *arguments],
                    capture_output=True,
                    text=True,
                    timeout=5,
                    check=False,
                )
                self.assertEqual(result.returncode, 4, result.stderr)

    def test_copy_reports_flush_failure(self):
        result = self.shell(
            "mnt=unused; cp() { return 0; }; sync() { return 1; }; copy_image image"
        )
        self.assertEqual(result.returncode, 1)

    def test_both_images_are_validated_before_any_flash(self):
        result = self.shell(
            """
IN_NIX_SHELL=1
cargo() { printf '%s\\n' \\
  '{"reason":"compiler-artifact","target":{"name":"moergo-control"},"executable":"control"}' \\
  '{"reason":"compiler-artifact","target":{"name":"xtask"},"executable":"xtask_mock"}' \\
  '{"reason":"build-finished","success":true}'; }
xtask_mock() { echo "validate $4"; [ "$2" != "$recovery" ]; }
flash() { echo UNEXPECTED_FLASH; }
recovery=$2
moergo_flash_main go60 "$1" --recover "$2" --peripheral --no-watch
""",
            str(LIBRARY),
            str(ROOT / "README.md"),
        )
        self.assertEqual(result.returncode, 4, result.stderr)
        self.assertEqual(
            result.stdout.splitlines(), ["validate 980ab007", "validate 980ab007"]
        )


if __name__ == "__main__":
    unittest.main()
