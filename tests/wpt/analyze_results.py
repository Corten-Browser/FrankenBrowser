#!/usr/bin/env python3
"""
Analyze Web Platform Test results for FrankenBrowser

This script parses WPT results and generates a summary report.
"""

import sys
import json
from pathlib import Path
from collections import defaultdict
from typing import Dict, List, Any


class WPTResultsAnalyzer:
    """Analyze WPT test results"""

    def __init__(self, results_file: str):
        """Initialize analyzer with results file"""
        self.results_file = Path(results_file)
        self.results = []
        self.summary = {
            "total": 0,
            "passed": 0,
            "failed": 0,
            "errors": 0,
            "timeouts": 0,
            "skipped": 0,
        }
        self.by_category = defaultdict(lambda: {
            "total": 0,
            "passed": 0,
            "failed": 0,
        })

    def load_results(self):
        """Load results from JSON file"""
        if not self.results_file.exists():
            print(f"Error: Results file not found: {self.results_file}")
            return False

        try:
            with open(self.results_file, 'r') as f:
                for line in f:
                    if line.strip():
                        try:
                            result = json.loads(line)
                            self.results.append(result)
                        except json.JSONDecodeError:
                            continue
            return True
        except Exception as e:
            print(f"Error loading results: {e}")
            return False

    def analyze(self):
        """Analyze the results"""
        for result in self.results:
            if result.get("action") == "test_status":
                status = result.get("status")
                test_path = result.get("test", "")

                # Update summary
                self.summary["total"] += 1

                if status == "PASS":
                    self.summary["passed"] += 1
                elif status == "FAIL":
                    self.summary["failed"] += 1
                elif status == "ERROR":
                    self.summary["errors"] += 1
                elif status == "TIMEOUT":
                    self.summary["timeouts"] += 1
                elif status == "SKIP":
                    self.summary["skipped"] += 1

                # Categorize by path
                category = self._get_category(test_path)
                self.by_category[category]["total"] += 1
                if status == "PASS":
                    self.by_category[category]["passed"] += 1
                elif status in ["FAIL", "ERROR", "TIMEOUT"]:
                    self.by_category[category]["failed"] += 1

    def _get_category(self, test_path: str) -> str:
        """Extract category from test path"""
        if not test_path:
            return "unknown"

        parts = test_path.strip("/").split("/")
        if len(parts) > 0:
            return parts[0]
        return "unknown"

    def calculate_pass_rate(self) -> float:
        """Calculate overall pass rate"""
        total = self.summary["total"]
        if total == 0:
            return 0.0
        return (self.summary["passed"] / total) * 100

    def print_summary(self):
        """Print summary report"""
        print("\n" + "=" * 70)
        print("WPT Results Summary")
        print("=" * 70)
        print(f"\nTotal Tests: {self.summary['total']}")
        print(f"  ✓ Passed:   {self.summary['passed']}")
        print(f"  ✗ Failed:   {self.summary['failed']}")
        print(f"  ! Errors:   {self.summary['errors']}")
        print(f"  ⏱ Timeouts: {self.summary['timeouts']}")
        print(f"  - Skipped:  {self.summary['skipped']}")
        print()

        pass_rate = self.calculate_pass_rate()
        print(f"Pass Rate: {pass_rate:.1f}%")

        # Pass rate assessment
        if pass_rate >= 40:
            print("✓ MEETS Phase 1 target (40%)")
        else:
            needed = (0.4 * self.summary['total']) - self.summary['passed']
            print(f"⚠ Below Phase 1 target (need {int(needed)} more passing tests)")

        print("\n" + "=" * 70)
        print("Results by Category")
        print("=" * 70)
        print(f"{'Category':<30} {'Tests':>8} {'Passed':>8} {'Rate':>8}")
        print("-" * 70)

        for category in sorted(self.by_category.keys()):
            stats = self.by_category[category]
            total = stats["total"]
            passed = stats["passed"]
            rate = (passed / total * 100) if total > 0 else 0

            print(f"{category:<30} {total:>8} {passed:>8} {rate:>7.1f}%")

        print("=" * 70)

    def generate_report(self, output_file: str = None):
        """Generate detailed report"""
        report = {
            "summary": self.summary,
            "pass_rate": self.calculate_pass_rate(),
            "by_category": dict(self.by_category),
            "phase1_target": 40.0,
            "meets_target": self.calculate_pass_rate() >= 40.0,
        }

        if output_file:
            with open(output_file, 'w') as f:
                json.dump(report, f, indent=2)
            print(f"\nDetailed report saved to: {output_file}")

        return report


def main():
    """Main entry point"""
    if len(sys.argv) < 2:
        print("Usage: analyze_results.py <results_file.json> [output_report.json]")
        sys.exit(1)

    results_file = sys.argv[1]
    output_file = sys.argv[2] if len(sys.argv) > 2 else None

    analyzer = WPTResultsAnalyzer(results_file)

    if not analyzer.load_results():
        sys.exit(1)

    analyzer.analyze()
    analyzer.print_summary()

    if output_file:
        analyzer.generate_report(output_file)

    # Exit with non-zero if below Phase 1 target
    pass_rate = analyzer.calculate_pass_rate()
    sys.exit(0 if pass_rate >= 40.0 else 1)


if __name__ == "__main__":
    main()
