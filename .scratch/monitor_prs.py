#!/usr/bin/env python3
"""
Code Review Monitor Agent - Monitors GitHub PRs and runs two-axis code reviews.
"""

import json
import subprocess
import time
import os
import sys
import re
from datetime import datetime
from pathlib import Path

REPO_PATH = Path("/home/medo/Desktop/projects/tunisia-career-scout")
REVIEWS_DIR = REPO_PATH / ".scratch" / "reviews"
KNOWN_PRS_FILE = REVIEWS_DIR / "known_prs.json"
CHECK_INTERVAL = 300  # 5 minutes

# Standards documents to check against
STANDARDS_FILES = {
    "CLAUDE.md": REPO_PATH / "CLAUDE.md",
    "GLOSSARY.md": REPO_PATH / "GLOSSARY.md",
    "ADR-LOG.md": REPO_PATH / "ADR-LOG.md",
    "SPEC.md": REPO_PATH / "specs" / "SPEC.md",
    "Cargo.toml": REPO_PATH / "app" / "Cargo.toml",
    "tsconfig.json": REPO_PATH / "app" / "frontend" / "tsconfig.json",
}

def load_standards_content():
    """Load all standards documents for reference during review."""
    content = {}
    for name, path in STANDARDS_FILES.items():
        if path.exists():
            content[name] = path.read_text()
        else:
            content[name] = f"[FILE NOT FOUND: {path}]"
    return content

STANDARDS_CONTENT = load_standards_content()

def run_cmd(cmd, cwd=None):
    """Run a command and return (stdout, stderr, returncode)."""
    result = subprocess.run(cmd, shell=True, capture_output=True, text=True, cwd=cwd or REPO_PATH)
    return result.stdout, result.stderr, result.returncode

def get_open_prs():
    """Get list of open PRs with their numbers and titles."""
    stdout, stderr, code = run_cmd("gh pr list --state=open --json number,title,headRefName,baseRefName,author,createdAt,url")
    if code != 0:
        print(f"Error getting PRs: {stderr}", file=sys.stderr)
        return []
    return json.loads(stdout)

def get_pr_diff(pr_number):
    """Get the diff for a PR."""
    stdout, stderr, code = run_cmd(f"gh pr diff {pr_number}")
    if code != 0:
        print(f"Error getting diff for PR #{pr_number}: {stderr}", file=sys.stderr)
        return None
    return stdout

def get_pr_files(pr_number):
    """Get list of files changed in a PR."""
    stdout, stderr, code = run_cmd(f"gh pr view {pr_number} --json files --jq '.files[] | .path'")
    if code != 0:
        return []
    return stdout.strip().split('\n') if stdout.strip() else []

def get_pr_commits(pr_number):
    """Get commits in a PR."""
    stdout, stderr, code = run_cmd(f"gh pr view {pr_number} --json commits --jq '.commits[] | .messageHeadline'")
    if code != 0:
        return []
    return stdout.strip().split('\n') if stdout.strip() else []

def load_known_prs():
    """Load known PR numbers from file."""
    if KNOWN_PRS_FILE.exists():
        try:
            return set(json.loads(KNOWN_PRS_FILE.read_text()))
        except:
            return set()
    return set()

def save_known_prs(prs):
    """Save known PR numbers to file."""
    KNOWN_PRS_FILE.write_text(json.dumps(list(prs)))

def check_fowler_smells(diff, files):
    """Check for common Fowler code smells in the diff."""
    findings = []
    
    lines = diff.split('\n')
    added_lines = [line[1:] for line in lines if line.startswith('+') and not line.startswith('+++')]
    
    # Check for magic numbers
    import re
    for i, line in enumerate(added_lines):
        if re.search(r'\b\d{3,}\b', line) and not any(x in line for x in ['port', 'timeout', 'max_', 'MIN_', 'MAX_', 'status', 'code']):
            findings.append(f"Possible magic number in added line: {line.strip()}")
    
    # Check for long parameter lists (in function signatures)
    for i, line in enumerate(added_lines):
        if line.count(',') >= 4 and ('fn ' in line or 'function ' in line or 'def ' in line or '=>' in line):
            findings.append(f"Long parameter list detected: {line.strip()[:100]}")
    
    # Check for comments that explain "what" not "why"
    for i, line in enumerate(added_lines):
        if line.strip().startswith('//') or line.strip().startswith('#') or line.strip().startswith('/*'):
            comment = line.strip()
            if any(word in comment.lower() for word in ['increment', 'decrement', 'set ', 'get ', 'assign', 'initialize', 'create new']):
                findings.append(f"Comment explains 'what' not 'why': {comment[:100]}")
    
    return findings

def check_naming_conventions(diff, files):
    """Check naming conventions per language standards."""
    findings = []
    
    for file in files:
        is_rust = file.endswith('.rs')
        is_ts = file.endswith('.ts') or file.endswith('.svelte')
        is_typst = file.endswith('.typ')
        
        if not (is_rust or is_ts or is_typst):
            continue
            
        # Get added lines for this file
        in_file = False
        file_lines = []
        for line in diff.split('\n'):
            if line.startswith('+++') and file in line:
                in_file = True
                continue
            elif line.startswith('---'):
                in_file = False
            elif in_file and line.startswith('+'):
                file_lines.append(line[1:])
        
        if is_rust:
            # Check snake_case for functions/variables, PascalCase for types
            import re
            for line in file_lines:
                # Check for camelCase in Rust (should be snake_case)
                if re.search(r'\b[a-z]+[A-Z][a-z]+\b', line) and 'fn ' in line:
                    findings.append(f"Rust: camelCase function name in {file}: {line.strip()[:80]}")
                # Check for SCREAMING_SNAKE_CASE for non-const
                if re.search(r'\b[A-Z_]{3,}\b', line) and 'const' not in line and 'static' not in line:
                    findings.append(f"Rust: SCREAMING_SNAKE_CASE for non-const in {file}: {line.strip()[:80]}")
                    
        elif is_ts:
            # Check camelCase for variables/functions, PascalCase for types/components
            import re
            for line in file_lines:
                # Check for snake_case in TS (should be camelCase)
                if re.search(r'\b[a-z]+_[a-z]+\b', line) and ('const ' in line or 'let ' in line or 'function ' in line or '=>' in line):
                    findings.append(f"TypeScript: snake_case variable/function in {file}: {line.strip()[:80]}")
                # Check for non-PascalCase components
                if 'export' in line and 'component' in line.lower():
                    match = re.search(r'(?:class|function|const)\s+([a-z][a-zA-Z]*)', line)
                    if match:
                        findings.append(f"TypeScript: Component not PascalCase in {file}: {line.strip()[:80]}")
    
    return findings

def check_rust_conventions(diff, files, cargo_content):
    """Check Rust-specific conventions from Cargo.toml."""
    findings = []
    
    # Check edition
    if 'edition = "2021"' not in cargo_content and 'edition = "2024"' not in cargo_content:
        findings.append("Cargo.toml: Consider updating to edition 2021 or 2024")
    
    # Check for common lints
    for file in files:
        if not file.endswith('.rs'):
            continue
            
        in_file = False
        file_lines = []
        for line in diff.split('\n'):
            if line.startswith('+++') and file in line:
                in_file = True
                continue
            elif line.startswith('---'):
                in_file = False
            elif in_file and line.startswith('+'):
                file_lines.append(line[1:])
        
        # Check for unwrap/expect without context
        for line in file_lines:
            if '.unwrap()' in line and '//' not in line.split('.unwrap()')[0]:
                findings.append(f"Rust: .unwrap() without context in {file}: {line.strip()[:80]}")
            if '.expect(' in line:
                findings.append(f"Rust: .expect() used in {file}: {line.strip()[:80]}")
    
    return findings

def check_ts_conventions(diff, files, tsconfig_content):
    """Check TypeScript-specific conventions from tsconfig.json."""
    findings = []
    
    # Check strict mode is enabled
    if '"strict": true' not in tsconfig_content and "'strict': true" not in tsconfig_content:
        findings.append("tsconfig.json: strict mode not enabled")
    
    for file in files:
        if not (file.endswith('.ts') or file.endswith('.svelte')):
            continue
            
        in_file = False
        file_lines = []
        for line in diff.split('\n'):
            if line.startswith('+++') and file in line:
                in_file = True
                continue
            elif line.startswith('---'):
                in_file = False
            elif in_file and line.startswith('+'):
                file_lines.append(line[1:])
        
        # Check for any type
        for line in file_lines:
            if ': any' in line or '<any>' in line:
                findings.append(f"TypeScript: 'any' type used in {file}: {line.strip()[:80]}")
            if '@ts-ignore' in line or '@ts-expect-error' in line:
                findings.append(f"TypeScript: ts-ignore used in {file}: {line.strip()[:80]}")
    
    return findings

def check_spec_compliance(diff, files, spec_content, pr_commits):
    """Check against SPEC.md requirements."""
    findings = []
    
    # Check for architectural patterns from SPEC
    # Repository pattern
    repo_pattern_mentioned = 'Repository' in spec_content or 'repository' in spec_content.lower()
    
    # Two-pass extraction
    two_pass_mentioned = 'Two-Pass' in spec_content or 'two-pass' in spec_content.lower() or 'RuleExtractor' in spec_content
    
    # Check diff for compliance
    for file in files:
        in_file = False
        file_lines = []
        for line in diff.split('\n'):
            if line.startswith('+++') and file in line:
                in_file = True
                continue
            elif line.startswith('---'):
                in_file = False
            elif in_file and line.startswith('+'):
                file_lines.append(line[1:])
        
        # Check if new files follow expected structure
        if file.endswith('.rs') and 'src/' in file:
            # Should follow module structure
            if 'scrapers' in file and 'adapter' not in file.lower() and 'pipeline' not in file.lower():
                findings.append(f"Spec: New scraper file {file} - ensure it follows SourceAdapter trait pattern")
            if 'models' in file:
                findings.append(f"Spec: New model in {file} - ensure it matches SPEC.md domain models (JobPosting, Source, CVData, etc.)")
            if 'commands' in file:
                findings.append(f"Spec: New command in {file} - ensure it follows Tauri command pattern with proper error handling")
        
        # Check for SQLx usage
        if file.endswith('.rs') and 'sqlx' in ''.join(file_lines).lower():
            if 'query!' not in ''.join(file_lines) and 'query_as!' not in ''.join(file_lines):
                findings.append(f"Spec: SQL usage in {file} - prefer compile-time checked query!/query_as! macros")
        
        # Check for proper error handling
        if file.endswith('.rs'):
            has_result = any('Result<' in line for line in file_lines)
            has_anyhow = any('anyhow' in line for line in file_lines)
            has_thiserror = any('thiserror' in line for line in file_lines)
            if not (has_result or has_anyhow or has_thiserror) and len(file_lines) > 20:
                findings.append(f"Spec: {file} - consider proper error handling with Result/anyhow/thiserror per SPEC")
    
    # Check commits reference issues
    issue_refs = []
    for commit in pr_commits:
        matches = re.findall(r'#(\d+)', commit)
        issue_refs.extend(matches)
    
    if not issue_refs:
        findings.append("Spec: No GitHub issue references found in commit messages - link PR to issue(s)")
    
    return findings

def check_adr_compliance(diff, files, adr_content):
    """Check against ADR decisions."""
    findings = []
    
    # ADR-0009: Repository Pattern
    if 'Repository' in adr_content:
        for file in files:
            if file.endswith('.rs') and 'repository' in file.lower():
                # Good - follows repository pattern
                pass
            elif file.endswith('.rs') and ('db' in file or 'database' in file or 'sql' in file):
                if 'repository' not in ''.join([l for l in diff.split('\n') if l.startswith('+')]).lower():
                    findings.append(f"ADR-0009: {file} - consider using Repository pattern for data access")
    
    # ADR-0007: Two-Pass Extraction
    if 'Two-Pass' in adr_content or 'RuleExtractor' in adr_content:
        for file in files:
            if 'extractor' in file.lower() or 'extraction' in file.lower():
                in_file = False
                file_lines = []
                for line in diff.split('\n'):
                    if line.startswith('+++') and file in line:
                        in_file = True
                        continue
                    elif line.startswith('---'):
                        in_file = False
                    elif in_file and line.startswith('+'):
                        file_lines.append(line[1:])
                
                content = ''.join(file_lines)
                if 'RuleExtractor' not in content and 'LLMExtractor' not in content:
                    findings.append(f"ADR-0007: {file} - ensure extraction follows two-pass (RuleExtractor + LLMExtractor) pattern")
    
    # ADR-0002: SQLx for database
    for file in files:
        if file.endswith('.rs'):
            in_file = False
            file_lines = []
            for line in diff.split('\n'):
                if line.startswith('+++') and file in line:
                    in_file = True
                    continue
                elif line.startswith('---'):
                    in_file = False
                elif in_file and line.startswith('+'):
                    file_lines.append(line[1:])
            
            content = ''.join(file_lines)
            if 'sqlx' in content.lower() and 'migrate' in content.lower():
                findings.append(f"ADR-0002: {file} - ensure migrations are in app/migrations/ and use sqlx::migrate!")
    
    return findings

def run_standards_review(pr_number, diff, files):
    """Run the Standards axis review."""
    findings = []
    
    findings.extend(check_fowler_smells(diff, files))
    findings.extend(check_naming_conventions(diff, files))
    findings.extend(check_rust_conventions(diff, files, STANDARDS_CONTENT.get("Cargo.toml", "")))
    findings.extend(check_ts_conventions(diff, files, STANDARDS_CONTENT.get("tsconfig.json", "")))
    findings.extend(check_adr_compliance(diff, files, STANDARDS_CONTENT.get("ADR-LOG.md", "")))
    
    return findings

def run_spec_review(pr_number, diff, files, pr_commits):
    """Run the Spec axis review."""
    findings = []
    
    findings.extend(check_spec_compliance(diff, files, STANDARDS_CONTENT.get("SPEC.md", ""), pr_commits))
    
    return findings

def generate_review_report(pr_number, pr_title, standards_findings, spec_findings):
    """Generate the review report in the required format."""
    now = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
    
    report = f"""# Code Review for PR #{pr_number}: {pr_title}
**Reviewed at**: {now}
**Reviewer**: Code Review Monitor Agent

## Standards
"""
    
    if standards_findings:
        for i, finding in enumerate(standards_findings, 1):
            report += f"{i}. {finding}\n"
    else:
        report += "No standards violations found.\n"
    
    report += "\n## Spec\n"
    
    if spec_findings:
        for i, finding in enumerate(spec_findings, 1):
            report += f"{i}. {finding}\n"
    else:
        report += "No spec violations found.\n"
    
    # Summary
    std_count = len(standards_findings)
    spec_count = len(spec_findings)
    
    std_worst = "none" if std_count == 0 else "multiple" if std_count > 3 else f"{std_count} finding(s)"
    spec_worst = "none" if spec_count == 0 else "multiple" if spec_count > 3 else f"{spec_count} finding(s)"
    
    report += f"""
## Summary
Standards: {std_count} findings (worst: {std_worst})
Spec: {spec_count} findings (worst: {spec_worst})
"""
    
    return report

def save_review(pr_number, report):
    """Save review to .scratch/reviews/"""
    timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
    filename = REVIEWS_DIR / f"pr_{pr_number}_{timestamp}.md"
    filename.write_text(report)
    return filename

def post_review_comment(pr_number, report):
    """Post review as PR comment."""
    # Escape for shell
    escaped = report.replace('"', '\\"').replace('$', '\\$').replace('`', '\\`')
    stdout, stderr, code = run_cmd(f'gh pr comment {pr_number} --body "{escaped}"')
    if code != 0:
        print(f"Failed to post comment: {stderr}", file=sys.stderr)
        return False
    return True

def review_pr(pr_number, pr_title):
    """Run complete review for a PR."""
    print(f"\n{'='*60}")
    print(f"Reviewing PR #{pr_number}: {pr_title}")
    print(f"{'='*60}")
    
    # Get PR data
    diff = get_pr_diff(pr_number)
    if not diff:
        print("  No diff available")
        return
    
    files = get_pr_files(pr_number)
    commits = get_pr_commits(pr_number)
    
    print(f"  Files changed: {len(files)}")
    print(f"  Commits: {len(commits)}")
    
    # Run reviews
    print("  Running Standards review...")
    standards_findings = run_standards_review(pr_number, diff, files)
    
    print("  Running Spec review...")
    spec_findings = run_spec_review(pr_number, diff, files, commits)
    
    # Generate report
    report = generate_review_report(pr_number, pr_title, standards_findings, spec_findings)
    
    # Save to file
    review_file = save_review(pr_number, report)
    print(f"  Review saved to: {review_file}")
    
    # Post as comment
    print("  Posting review comment...")
    if post_review_comment(pr_number, report):
        print("  Review posted as PR comment")
    else:
        print("  Failed to post comment (saved to file)")
    
    # Print summary
    print(f"\n  Standards: {len(standards_findings)} findings")
    print(f"  Spec: {len(spec_findings)} findings")

def main():
    """Main monitoring loop."""
    print("Starting Code Review Monitor Agent")
    print(f"Repository: {REPO_PATH}")
    print(f"Check interval: {CHECK_INTERVAL} seconds")
    print(f"Reviews directory: {REVIEWS_DIR}")
    print("-" * 60)
    
    known_prs = load_known_prs()
    print(f"Known PRs: {known_prs}")
    
    iteration = 0
    while True:
        iteration += 1
        print(f"\n[{datetime.now().strftime('%H:%M:%S')}] Check #{iteration}")
        
        try:
            open_prs = get_open_prs()
            current_pr_numbers = {pr['number'] for pr in open_prs}
            
            # Find new PRs
            new_prs = current_pr_numbers - known_prs
            
            if new_prs:
                print(f"  Found {len(new_prs)} new PR(s): {new_prs}")
                for pr in open_prs:
                    if pr['number'] in new_prs:
                        review_pr(pr['number'], pr['title'])
                
                # Update known PRs
                known_prs = current_pr_numbers
                save_known_prs(known_prs)
            else:
                print(f"  No new PRs. Open PRs: {sorted(current_pr_numbers)}")
                
        except Exception as e:
            print(f"  Error during check: {e}", file=sys.stderr)
            import traceback
            traceback.print_exc()
        
        print(f"  Sleeping {CHECK_INTERVAL}s...")
        time.sleep(CHECK_INTERVAL)

if __name__ == "__main__":
    main()