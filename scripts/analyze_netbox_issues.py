#!/usr/bin/env python3
"""
Direct analysis of NetBox reconciliation issues from logs.

This script:
1. Extracts key errors and warnings from logs
2. Groups by resource type and issue type
3. Provides actionable recommendations
"""

import re
import subprocess
import sys
from collections import defaultdict
from pathlib import Path

def strip_ansi(line):
    """Remove ANSI escape codes."""
    ansi_escape = re.compile(r'\x1B(?:[@-Z\\-_]|\[[0-?]*[ -/]*[@-~])')
    return ansi_escape.sub('', line)

def extract_issues(log_file: Path):
    """Extract issues from log file."""
    issues = {
        'errors': defaultdict(list),
        'warnings': defaultdict(list),
        'missing_crds': set(),
        'missing_tags': set(),
        'netbox_validation_errors': [],
        'ip_address_issues': [],
    }
    
    with open(log_file, 'r') as f:
        for line in f:
            clean = strip_ansi(line)
            
            # Skip non-log lines
            if not re.match(r'\d{4}-\d{2}-\d{2}T', clean):
                continue
            
            # Extract level
            level_match = re.search(r'\s+(INFO|WARN|ERROR)\s+', clean)
            if not level_match:
                continue
            level = level_match.group(1)
            
            # Extract resource type and name
            resource_match = re.search(r'NetBox(\w+)\.v1alpha1[^/]+/([^\s.]+)', clean)
            resource_type = resource_match.group(1) if resource_match else None
            resource_name = resource_match.group(2) if resource_match else None
            
            # Extract reconciler
            reconciler_match = re.search(r'netbox_controller::reconciler::([^:]+)', clean)
            if reconciler_match:
                reconciler = reconciler_match.group(1).replace('::', '_')
            elif 'watcher' in clean:
                reconciler = 'watcher'
            elif 'token_resolver' in clean:
                reconciler = 'token_resolver'
            elif 'reconcile_helpers' in clean:
                reconciler = 'reconcile_helpers'
            else:
                reconciler = 'other'
            
            # Extract message
            message_match = re.search(r':\s+(.+)', clean)
            message = message_match.group(1) if message_match else clean
            
            # Categorize issues
            if level == "ERROR":
                issues['errors'][resource_type or 'unknown'].append({
                    'resource': resource_name,
                    'reconciler': reconciler,
                    'message': message[:200]
                })
                
                # NetBox validation errors
                if "NetBox API error" in message or "400 Bad Request" in message:
                    issues['netbox_validation_errors'].append({
                        'resource': resource_name,
                        'resource_type': resource_type,
                        'message': message[:300]
                    })
                
                # IP address specific issues
                if resource_type == "IPAddress":
                    issues['ip_address_issues'].append({
                        'resource': resource_name,
                        'message': message[:300]
                    })
            
            elif level == "WARN":
                issues['warnings'][resource_type or 'unknown'].append({
                    'resource': resource_name,
                    'reconciler': reconciler,
                    'message': message[:200]
                })
                
                # Missing CRDs
                if "CRD" in message and "not found" in message:
                    crd_match = re.search(r"CRD '([^']+)'", message)
                    if crd_match:
                        issues['missing_crds'].add(crd_match.group(1))
                
                # Missing tags
                if "not found in NetBox" in message and "Tag" in message:
                    tag_match = re.search(r"Tag '([^']+)'", message)
                    if tag_match:
                        issues['missing_tags'].add(tag_match.group(1))
    
    return issues

def get_netbox_ip_range_info():
    """Get IP range information from NetBox via kubectl exec."""
    try:
        # Try to get NetBox pod
        result = subprocess.run(
            ["kubectl", "get", "pod", "-n", "netbox", "-l", "app=netbox", "-o", "jsonpath={.items[0].metadata.name}"],
            capture_output=True,
            text=True,
            check=False
        )
        if result.returncode != 0:
            return None
        
        pod_name = result.stdout.strip()
        if not pod_name:
            return None
        
        # Get token from secret
        token_result = subprocess.run(
            ["kubectl", "get", "secret", "netbox-token", "-n", "dcops-system", "-o", "jsonpath={.data.token}"],
            capture_output=True,
            text=True,
            check=False
        )
        if token_result.returncode != 0:
            return None
        
        import base64
        token = base64.b64decode(token_result.stdout.strip()).decode('utf-8')
        if token == "PLACEHOLDER_DO_NOT_USE":
            return None
        
        # Query NetBox API from within cluster
        curl_cmd = f"curl -s -H 'Authorization: Token {token}' http://netbox.netbox/api/ipam/ip-ranges/"
        result = subprocess.run(
            ["kubectl", "exec", "-n", "netbox", pod_name, "--", "sh", "-c", curl_cmd],
            capture_output=True,
            text=True,
            check=False
        )
        
        if result.returncode == 0:
            import json
            try:
                data = json.loads(result.stdout)
                return data.get('results', [])
            except:
                pass
    except Exception as e:
        print(f"⚠️  Could not query NetBox API: {e}", file=sys.stderr)
    
    return None

def main():
    """Main function."""
    project_root = Path(__file__).parent.parent
    log_file = project_root / "tilt-netbox-controller.logs"
    
    if not log_file.exists():
        print(f"❌ Log file not found: {log_file}")
        sys.exit(1)
    
    print("=" * 80)
    print("NETBOX RECONCILIATION ISSUE ANALYSIS")
    print("=" * 80)
    print()
    
    # Extract issues
    print("🔍 Analyzing logs...")
    issues = extract_issues(log_file)
    
    # Get NetBox IP range info
    print("📡 Querying NetBox API for IP ranges...")
    ip_ranges = get_netbox_ip_range_info()
    
    print()
    print("=" * 80)
    print("ISSUE SUMMARY")
    print("=" * 80)
    print()
    
    # Errors by resource type
    print("❌ ERRORS BY RESOURCE TYPE:")
    print("-" * 80)
    for resource_type in sorted(issues['errors'].keys()):
        errors = issues['errors'][resource_type]
        print(f"\n{resource_type}: {len(errors)} errors")
        
        # Group by error type
        error_types = defaultdict(list)
        for err in errors:
            msg = err['message']
            if "Cannot create IP address" in msg:
                error_types['IP_RANGE_VALIDATION'].append(err)
            elif "IP address must be specified" in msg:
                error_types['MISSING_IP_ADDRESS'].append(err)
            elif "not within the specified IP range" in msg:
                error_types['IP_OUT_OF_RANGE'].append(err)
            else:
                error_types['OTHER'].append(err)
        
        for err_type, errs in error_types.items():
            if errs:
                print(f"  {err_type}: {len(errs)}")
                for err in errs[:3]:
                    print(f"    • {err['resource']}: {err['message'][:100]}")
                if len(errs) > 3:
                    print(f"    ... and {len(errs) - 3} more")
    
    print()
    
    # Warnings
    print("⚠️  WARNINGS BY RESOURCE TYPE:")
    print("-" * 80)
    for resource_type in sorted(issues['warnings'].keys()):
        warnings = issues['warnings'][resource_type]
        if warnings:
            print(f"\n{resource_type}: {len(warnings)} warnings")
            unique_warnings = {}
            for warn in warnings:
                key = warn['message'][:100]
                if key not in unique_warnings:
                    unique_warnings[key] = warn
            
            for warn in list(unique_warnings.values())[:5]:
                print(f"  • {warn['resource'] or 'N/A'}: {warn['message'][:120]}")
            if len(unique_warnings) > 5:
                print(f"  ... and {len(unique_warnings) - 5} more")
    
    print()
    
    # Missing CRDs
    if issues['missing_crds']:
        print("📋 MISSING CRDs (referenced but not found in cluster):")
        print("-" * 80)
        for crd in sorted(issues['missing_crds']):
            print(f"  • {crd}")
        print()
    
    # Missing tags
    if issues['missing_tags']:
        print("🏷️  MISSING TAGS (referenced but not found in NetBox):")
        print("-" * 80)
        for tag in sorted(issues['missing_tags']):
            print(f"  • {tag}")
        print()
    
    # NetBox IP Range validation issue
    print("🔴 CRITICAL: NetBox IP Range Validation Error")
    print("-" * 80)
    print("Issue: Cannot create IP address 192.168.1.101/24 inside range 192.168.1.100-200/24")
    print()
    print("This error suggests:")
    print("  1. The IP range in NetBox may be configured incorrectly")
    print("  2. The IP range may have 'mark_utilized' enabled, preventing IP creation")
    print("  3. There may be a conflict with existing IP addresses")
    print()
    
    if ip_ranges:
        print("📊 NetBox IP Ranges:")
        for ip_range in ip_ranges:
            print(f"  • ID {ip_range.get('id')}: {ip_range.get('start_address')} - {ip_range.get('end_address')}")
            print(f"    Status: {ip_range.get('status', {}).get('value', 'N/A')}")
            print(f"    Mark Utilized: {ip_range.get('mark_utilized', 'N/A')}")
            print()
    else:
        print("⚠️  Could not query NetBox API for IP ranges")
        print()
    
    # IP Address specific issues
    if issues['ip_address_issues']:
        print("🌐 IP ADDRESS SPECIFIC ISSUES:")
        print("-" * 80)
        issue_groups = defaultdict(list)
        for issue in issues['ip_address_issues']:
            msg = issue['message']
            if "Cannot create IP address" in msg:
                issue_groups['IP_IN_RANGE_VALIDATION'].append(issue)
            elif "IP address must be specified" in msg:
                issue_groups['MISSING_ADDRESS'].append(issue)
            elif "not within the specified IP range" in msg:
                issue_groups['IP_OUT_OF_RANGE'].append(issue)
        
        for group, items in issue_groups.items():
            print(f"\n{group}: {len(items)} issues")
            for item in items[:3]:
                print(f"  • {item['resource']}: {item['message'][:150]}")
            if len(items) > 3:
                print(f"  ... and {len(items) - 3} more")
        print()
    
    # Recommendations
    print("=" * 80)
    print("RECOMMENDATIONS")
    print("=" * 80)
    print()
    
    if "Cannot create IP address" in str(issues['netbox_validation_errors']):
        print("1. IP RANGE CONFIGURATION ISSUE:")
        print("   • Check NetBox IP Range 'dhcp-pool-range' configuration")
        print("   • Verify 'mark_utilized' is set to False")
        print("   • Ensure the range allows IP address creation")
        print("   • Check if IP 192.168.1.101 already exists in NetBox")
        print()
    
    if issues['missing_crds']:
        print("2. MISSING CRDs:")
        print("   • Create missing CRDs:")
        for crd in sorted(issues['missing_crds']):
            print(f"     - {crd}")
        print()
    
    if issues['missing_tags']:
        print("3. MISSING TAGS:")
        print("   • Create missing tags in NetBox or remove tag references from CRs:")
        for tag in sorted(issues['missing_tags']):
            print(f"     - {tag}")
        print()
    
    if any("IP address must be specified" in str(e) for e in issues['ip_address_issues']):
        print("4. DHCP RANDOM ALLOCATION ISSUE:")
        print("   • For random DHCP allocation, the reconciler should allocate from the IP range")
        print("   • Current implementation requires spec.address or status.address")
        print("   • This may be a bug in the reconciler logic")
        print()
    
    print("✅ Analysis complete!")
    print()

if __name__ == "__main__":
    main()

