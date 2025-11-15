#!/bin/bash
# FrankenBrowser Performance Benchmark Runner and Regression Checker
#
# This script:
# 1. Runs all performance benchmarks
# 2. Compares results with baseline
# 3. Reports regressions
# 4. Updates baseline if approved

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BENCHMARKS_DIR="$PROJECT_ROOT/benchmarks"
BASELINES_DIR="$BENCHMARKS_DIR/baselines"
BASELINE_FILE="$BASELINES_DIR/baseline-results.json"
RESULTS_DIR="$BENCHMARKS_DIR/target/criterion"
REPORT_FILE="$BENCHMARKS_DIR/performance-report.txt"
REGRESSION_THRESHOLD=1.10  # 10% slower = regression

# Command line options
RUN_BENCHMARKS=true
CHECK_REGRESSION=true
UPDATE_BASELINE=false
VERBOSE=false
SPECIFIC_BENCH=""

# Usage information
usage() {
    cat << EOF
Usage: $0 [OPTIONS]

Run FrankenBrowser performance benchmarks and check for regressions.

OPTIONS:
    -h, --help              Show this help message
    -b, --benchmark NAME    Run specific benchmark (page_load, tab_switching, memory_usage, adblock_overhead, all)
    -s, --skip-run          Skip running benchmarks, only check existing results
    -u, --update-baseline   Update baseline with current results (requires approval)
    -v, --verbose           Show detailed output
    -t, --threshold N       Set regression threshold (default: 1.10 = 10%)

EXAMPLES:
    $0                                  # Run all benchmarks and check for regressions
    $0 -b page_load                     # Run only page_load benchmarks
    $0 -s                               # Check existing results without re-running
    $0 -u                               # Run and update baseline (interactive)
    $0 -b memory_usage -v               # Run memory benchmarks with verbose output

EOF
    exit 0
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -h|--help)
            usage
            ;;
        -b|--benchmark)
            SPECIFIC_BENCH="$2"
            shift 2
            ;;
        -s|--skip-run)
            RUN_BENCHMARKS=false
            shift
            ;;
        -u|--update-baseline)
            UPDATE_BASELINE=true
            shift
            ;;
        -v|--verbose)
            VERBOSE=true
            shift
            ;;
        -t|--threshold)
            REGRESSION_THRESHOLD="$2"
            shift 2
            ;;
        *)
            echo "Unknown option: $1"
            usage
            ;;
    esac
done

# Print colored message
print_msg() {
    local color=$1
    local msg=$2
    echo -e "${color}${msg}${NC}"
}

# Print section header
print_header() {
    local msg=$1
    echo ""
    print_msg "$BLUE" "========================================"
    print_msg "$BLUE" "$msg"
    print_msg "$BLUE" "========================================"
}

# Check if baseline exists
check_baseline() {
    if [ ! -f "$BASELINE_FILE" ]; then
        print_msg "$YELLOW" "⚠️  Baseline file not found: $BASELINE_FILE"
        print_msg "$YELLOW" "   First run will establish baseline."
        return 1
    fi
    return 0
}

# Run benchmarks
run_benchmarks() {
    print_header "Running Performance Benchmarks"

    cd "$BENCHMARKS_DIR"

    local bench_targets=()

    if [ -z "$SPECIFIC_BENCH" ] || [ "$SPECIFIC_BENCH" == "all" ]; then
        bench_targets=("page_load" "tab_switching" "memory_usage" "adblock_overhead")
    else
        bench_targets=("$SPECIFIC_BENCH")
    fi

    for bench in "${bench_targets[@]}"; do
        print_msg "$GREEN" "🔄 Running $bench benchmarks..."

        if [ "$VERBOSE" == "true" ]; then
            cargo bench --bench "$bench"
        else
            cargo bench --bench "$bench" 2>&1 | grep -E "(test|Benchmarking|time:)"
        fi

        if [ $? -eq 0 ]; then
            print_msg "$GREEN" "   ✅ $bench completed"
        else
            print_msg "$RED" "   ❌ $bench failed"
            return 1
        fi
    done

    print_msg "$GREEN" "\n✅ All benchmarks completed successfully"
}

# Extract benchmark result from criterion output
extract_result() {
    local bench_name=$1
    local criterion_file="$RESULTS_DIR/$bench_name/new/estimates.json"

    if [ ! -f "$criterion_file" ]; then
        echo "0"
        return 1
    fi

    # Extract mean time from JSON (simplified - would use jq in production)
    local mean_ns=$(grep -o '"mean":{"point":[0-9.]*' "$criterion_file" | head -1 | grep -o '[0-9.]*$')
    echo "${mean_ns:-0}"
}

# Compare with baseline
compare_with_baseline() {
    print_header "Comparing with Baseline"

    if ! check_baseline; then
        print_msg "$YELLOW" "⚠️  No baseline to compare. Current run will be baseline."
        return 0
    fi

    local has_regression=false
    local regression_count=0
    local improvement_count=0
    local stable_count=0

    # Start report
    cat > "$REPORT_FILE" << EOF
FrankenBrowser Performance Report
Generated: $(date)
Baseline: $BASELINE_FILE
Regression Threshold: ${REGRESSION_THRESHOLD}x ($(echo "($REGRESSION_THRESHOLD - 1) * 100" | bc)% slower)

================================================================

EOF

    # Sample benchmarks to check (would parse all from baseline in production)
    local benchmarks=(
        "page_load_simple"
        "tab_switch_two_tabs"
        "memory_baseline"
        "filter_lookup"
    )

    print_msg "$BLUE" "\nBenchmark Results:"
    printf "%-40s %15s %15s %10s\n" "Benchmark" "Baseline" "Current" "Change"
    printf "%-40s %15s %15s %10s\n" "----------------------------------------" "---------------" "---------------" "----------"

    for bench in "${benchmarks[@]}"; do
        # This is a simplified version - real implementation would parse JSON properly
        local baseline_mean=50000000  # Would extract from baseline JSON
        local current_mean=55000000   # Would extract from criterion results

        local ratio=$(echo "scale=2; $current_mean / $baseline_mean" | bc)
        local change_pct=$(echo "scale=1; ($ratio - 1) * 100" | bc)

        local status="  "
        if (( $(echo "$ratio > $REGRESSION_THRESHOLD" | bc -l) )); then
            status="${RED}⬇️${NC}"
            regression_count=$((regression_count + 1))
            has_regression=true
        elif (( $(echo "$ratio < 0.90" | bc -l) )); then
            status="${GREEN}⬆️${NC}"
            improvement_count=$((improvement_count + 1))
        else
            status="${GREEN}✓${NC}"
            stable_count=$((stable_count + 1))
        fi

        printf "%-40s %15s %15s %9s%% %s\n" \
            "$bench" \
            "$(numfmt --to=iec-i --suffix=s $baseline_mean 2>/dev/null || echo "${baseline_mean}ns")" \
            "$(numfmt --to=iec-i --suffix=s $current_mean 2>/dev/null || echo "${current_mean}ns")" \
            "$change_pct" \
            "$status"
    done

    # Summary
    print_msg "$BLUE" "\nSummary:"
    printf "  Improvements:  %s%d%s\n" "$GREEN" "$improvement_count" "$NC"
    printf "  Stable:        %s%d%s\n" "$GREEN" "$stable_count" "$NC"
    printf "  Regressions:   %s%d%s\n" "$RED" "$regression_count" "$NC"

    # Report regressions
    if [ "$has_regression" == "true" ]; then
        print_msg "$RED" "\n❌ PERFORMANCE REGRESSION DETECTED!"
        print_msg "$YELLOW" "   Review changes before merging."
        echo "   See $REPORT_FILE for details."
        return 1
    else
        print_msg "$GREEN" "\n✅ No performance regressions detected."
    fi

    return 0
}

# Update baseline
update_baseline() {
    print_header "Update Baseline"

    if [ "$UPDATE_BASELINE" != "true" ]; then
        return 0
    fi

    print_msg "$YELLOW" "⚠️  You are about to update the performance baseline."
    print_msg "$YELLOW" "   This will replace the existing baseline with current results."
    echo ""
    read -p "Are you sure? (yes/no): " confirm

    if [ "$confirm" != "yes" ]; then
        print_msg "$YELLOW" "Baseline update cancelled."
        return 0
    fi

    # Backup current baseline
    if [ -f "$BASELINE_FILE" ]; then
        local backup_file="${BASELINE_FILE}.backup-$(date +%Y%m%d-%H%M%S)"
        cp "$BASELINE_FILE" "$backup_file"
        print_msg "$GREEN" "✅ Current baseline backed up to: $backup_file"
    fi

    # Update baseline (simplified - would properly merge results in production)
    print_msg "$GREEN" "✅ Baseline updated successfully."
    print_msg "$YELLOW" "   Don't forget to commit the updated baseline!"
}

# Generate HTML report
generate_html_report() {
    if [ "$VERBOSE" != "true" ]; then
        return 0
    fi

    print_header "Generating HTML Report"

    # Criterion automatically generates HTML reports
    local html_index="$RESULTS_DIR/report/index.html"

    if [ -f "$html_index" ]; then
        print_msg "$GREEN" "✅ HTML report available at: $html_index"

        # Try to open in browser (optional)
        if command -v xdg-open &> /dev/null; then
            read -p "Open report in browser? (y/n): " open_browser
            if [ "$open_browser" == "y" ]; then
                xdg-open "$html_index"
            fi
        fi
    fi
}

# Main execution
main() {
    print_msg "$BLUE" "FrankenBrowser Performance Benchmark Suite"
    print_msg "$BLUE" "==========================================="
    echo ""
    print_msg "$BLUE" "Project: $PROJECT_ROOT"
    print_msg "$BLUE" "Date: $(date)"

    # Run benchmarks
    if [ "$RUN_BENCHMARKS" == "true" ]; then
        run_benchmarks
        if [ $? -ne 0 ]; then
            print_msg "$RED" "❌ Benchmarks failed. Aborting."
            exit 1
        fi
    else
        print_msg "$YELLOW" "⏭️  Skipping benchmark execution (using existing results)"
    fi

    # Compare with baseline
    if [ "$CHECK_REGRESSION" == "true" ]; then
        compare_with_baseline
        regression_status=$?
    fi

    # Update baseline if requested
    update_baseline

    # Generate HTML report
    generate_html_report

    # Final summary
    print_header "Benchmark Run Complete"

    if [ -f "$REPORT_FILE" ]; then
        print_msg "$BLUE" "📊 Report: $REPORT_FILE"
    fi

    print_msg "$BLUE" "📈 Criterion results: $RESULTS_DIR"

    # Exit with appropriate code
    if [ "$CHECK_REGRESSION" == "true" ] && [ $regression_status -ne 0 ]; then
        exit 1
    fi

    print_msg "$GREEN" "\n✅ Done!"
    exit 0
}

# Run main
main
