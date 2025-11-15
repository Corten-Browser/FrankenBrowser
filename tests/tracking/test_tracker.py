#!/usr/bin/env python3
"""
Test tracking database manager for FrankenBrowser

This script manages the test tracking database, records test results,
and generates quality/performance reports.
"""

import sqlite3
import argparse
import json
import subprocess
from datetime import datetime
from pathlib import Path
from typing import Dict, List, Optional, Tuple

DATABASE_PATH = Path(__file__).parent / "test_results.db"
SCHEMA_PATH = Path(__file__).parent / "schema.sql"


class TestTracker:
    """Manages test tracking database"""

    def __init__(self, db_path: Path = DATABASE_PATH):
        self.db_path = db_path
        self.conn: Optional[sqlite3.Connection] = None

    def __enter__(self):
        self.conn = sqlite3.connect(self.db_path)
        self.conn.row_factory = sqlite3.Row
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        if self.conn:
            self.conn.close()

    def initialize(self):
        """Initialize database with schema"""
        if not SCHEMA_PATH.exists():
            raise FileNotFoundError(f"Schema file not found: {SCHEMA_PATH}")

        with open(SCHEMA_PATH, 'r') as f:
            schema_sql = f.read()

        self.conn.executescript(schema_sql)
        self.conn.commit()
        print(f"✓ Database initialized: {self.db_path}")

    def get_git_info(self) -> Tuple[str, str]:
        """Get current git commit hash and branch"""
        try:
            commit_hash = subprocess.check_output(
                ['git', 'rev-parse', 'HEAD'],
                stderr=subprocess.DEVNULL
            ).decode().strip()

            branch = subprocess.check_output(
                ['git', 'rev-parse', '--abbrev-ref', 'HEAD'],
                stderr=subprocess.DEVNULL
            ).decode().strip()

            return commit_hash[:12], branch
        except subprocess.CalledProcessError:
            return "unknown", "unknown"

    def record_test_run(
        self,
        component: str,
        test_suite: str,
        total: int,
        passed: int,
        failed: int,
        skipped: int = 0,
        ignored: int = 0,
        duration_ms: Optional[int] = None,
        runner: str = "local"
    ):
        """Record a test run"""
        commit_hash, branch = self.get_git_info()
        pass_rate = passed / total if total > 0 else 0.0

        cursor = self.conn.execute("""
            INSERT INTO test_runs (
                component, test_suite, total_tests, passed_tests,
                failed_tests, skipped_tests, ignored_tests, pass_rate,
                commit_hash, branch, duration_ms, runner
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        """, (
            component, test_suite, total, passed, failed, skipped,
            ignored, pass_rate, commit_hash, branch, duration_ms, runner
        ))

        self.conn.commit()
        print(f"✓ Recorded test run: {component}/{test_suite} - {passed}/{total} passing ({pass_rate:.1%})")
        return cursor.lastrowid

    def record_performance(
        self,
        benchmark: str,
        mean_ns: int,
        std_dev_ns: int,
        min_ns: int,
        max_ns: int,
        iterations: int,
        runner: str = "local"
    ):
        """Record benchmark performance"""
        commit_hash, branch = self.get_git_info()

        cursor = self.conn.execute("""
            INSERT INTO performance_results (
                benchmark, mean_time_ns, std_dev_ns, min_time_ns,
                max_time_ns, iterations, commit_hash, branch, runner
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        """, (
            benchmark, mean_ns, std_dev_ns, min_ns, max_ns,
            iterations, commit_hash, branch, runner
        ))

        self.conn.commit()
        mean_ms = mean_ns / 1_000_000
        print(f"✓ Recorded benchmark: {benchmark} - {mean_ms:.2f}ms avg")
        return cursor.lastrowid

    def record_quality_metric(
        self,
        component: str,
        metric_name: str,
        metric_value: float
    ):
        """Record a quality metric"""
        commit_hash, branch = self.get_git_info()

        cursor = self.conn.execute("""
            INSERT INTO quality_metrics (
                component, metric_name, metric_value,
                commit_hash, branch
            ) VALUES (?, ?, ?, ?, ?)
        """, (component, metric_name, metric_value, commit_hash, branch))

        self.conn.commit()
        print(f"✓ Recorded metric: {component}/{metric_name} = {metric_value}")
        return cursor.lastrowid

    def get_latest_results(self) -> List[Dict]:
        """Get latest test results for each component/suite"""
        cursor = self.conn.execute("""
            SELECT * FROM latest_test_results
            ORDER BY component, test_suite
        """)
        return [dict(row) for row in cursor.fetchall()]

    def get_test_trends(self, days: int = 30) -> List[Dict]:
        """Get test trends over specified days"""
        cursor = self.conn.execute("""
            SELECT
                component,
                test_suite,
                DATE(run_timestamp) as date,
                AVG(pass_rate) as avg_pass_rate,
                COUNT(*) as runs_count
            FROM test_runs
            WHERE run_timestamp >= datetime('now', '-' || ? || ' days')
            GROUP BY component, test_suite, DATE(run_timestamp)
            ORDER BY date DESC, component, test_suite
        """, (days,))
        return [dict(row) for row in cursor.fetchall()]

    def get_performance_trends(self, benchmark: Optional[str] = None, days: int = 30) -> List[Dict]:
        """Get performance trends"""
        if benchmark:
            cursor = self.conn.execute("""
                SELECT
                    benchmark,
                    DATE(run_timestamp) as date,
                    AVG(mean_time_ns) as avg_mean_ns,
                    MIN(min_time_ns) as best_time_ns,
                    MAX(max_time_ns) as worst_time_ns
                FROM performance_results
                WHERE benchmark = ? AND run_timestamp >= datetime('now', '-' || ? || ' days')
                GROUP BY benchmark, DATE(run_timestamp)
                ORDER BY date DESC
            """, (benchmark, days))
        else:
            cursor = self.conn.execute("""
                SELECT * FROM performance_trends
                LIMIT 100
            """)

        return [dict(row) for row in cursor.fetchall()]

    def generate_dashboard(self) -> str:
        """Generate markdown dashboard"""
        lines = []
        lines.append("# FrankenBrowser Test Dashboard\n")
        lines.append(f"**Generated**: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}\n")

        # Latest test results
        lines.append("## Latest Test Results\n")
        results = self.get_latest_results()

        if results:
            lines.append("| Component | Test Suite | Total | Passed | Failed | Pass Rate |")
            lines.append("|-----------|------------|-------|--------|--------|-----------|")

            for r in results:
                pass_rate = f"{r['pass_rate']:.1%}"
                status = "✅" if r['pass_rate'] >= 0.90 else "⚠️" if r['pass_rate'] >= 0.70 else "❌"
                lines.append(
                    f"| {r['component']} | {r['test_suite']} | {r['total_tests']} | "
                    f"{r['passed_tests']} | {r['failed_tests']} | {status} {pass_rate} |"
                )
        else:
            lines.append("*No test results recorded yet*\n")

        lines.append("")

        # Overall summary
        cursor = self.conn.execute("""
            SELECT
                SUM(total_tests) as total,
                SUM(passed_tests) as passed,
                SUM(failed_tests) as failed,
                AVG(pass_rate) as avg_pass_rate
            FROM latest_test_results
        """)
        summary = cursor.fetchone()

        if summary and summary['total']:
            lines.append("## Overall Summary\n")
            lines.append(f"- **Total Tests**: {summary['total']}")
            lines.append(f"- **Passing**: {summary['passed']}")
            lines.append(f"- **Failing**: {summary['failed']}")
            lines.append(f"- **Average Pass Rate**: {summary['avg_pass_rate']:.1%}\n")

        return "\n".join(lines)


def main():
    parser = argparse.ArgumentParser(description="FrankenBrowser test tracking")
    subparsers = parser.add_subparsers(dest='command', help='Commands')

    # Initialize command
    subparsers.add_parser('init', help='Initialize database')

    # Record test run
    record_parser = subparsers.add_parser('record', help='Record test run')
    record_parser.add_argument('component', help='Component name')
    record_parser.add_argument('suite', help='Test suite name')
    record_parser.add_argument('total', type=int, help='Total tests')
    record_parser.add_argument('passed', type=int, help='Passed tests')
    record_parser.add_argument('failed', type=int, help='Failed tests')
    record_parser.add_argument('--skipped', type=int, default=0)
    record_parser.add_argument('--ignored', type=int, default=0)
    record_parser.add_argument('--duration', type=int, help='Duration in ms')
    record_parser.add_argument('--runner', default='local')

    # Dashboard command
    dash_parser = subparsers.add_parser('dashboard', help='Generate dashboard')
    dash_parser.add_argument('--output', help='Output file')

    # Trends command
    trends_parser = subparsers.add_parser('trends', help='Show trends')
    trends_parser.add_argument('--days', type=int, default=30)

    args = parser.parse_args()

    with TestTracker() as tracker:
        if args.command == 'init':
            tracker.initialize()

        elif args.command == 'record':
            tracker.record_test_run(
                args.component,
                args.suite,
                args.total,
                args.passed,
                args.failed,
                args.skipped,
                args.ignored,
                args.duration,
                args.runner
            )

        elif args.command == 'dashboard':
            dashboard = tracker.generate_dashboard()
            if args.output:
                Path(args.output).write_text(dashboard)
                print(f"✓ Dashboard written to: {args.output}")
            else:
                print(dashboard)

        elif args.command == 'trends':
            trends = tracker.get_test_trends(args.days)
            print(f"\nTest Trends (last {args.days} days):\n")
            for t in trends[:20]:  # Show first 20
                print(f"  {t['date']} {t['component']}/{t['test_suite']}: {t['avg_pass_rate']:.1%} ({t['runs_count']} runs)")

        else:
            parser.print_help()


if __name__ == '__main__':
    main()
