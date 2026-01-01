#!/usr/bin/env python3
"""
Setup script for ASUS Ascent repository server.

This script helps set up an HTTP server to serve the ASUS Ascent ISO repository
for use in Docker builds and Kubernetes node provisioning.

Usage:
    python scripts/setup_asus_repo_server.py --repo-path /path/to/ASUS-Ascent-GX10-OS-7.2.3-3-20251007074705-arm64 --port 8000
"""

import argparse
import http.server
import socketserver
import os
import sys
from pathlib import Path


class ASUSRepoHTTPRequestHandler(http.server.SimpleHTTPRequestHandler):
    """Custom HTTP request handler for serving ASUS repository."""
    
    def end_headers(self):
        # Add CORS headers if needed
        self.send_header('Access-Control-Allow-Origin', '*')
        # Set correct MIME type for .deb files
        if self.path.endswith('.deb'):
            self.send_header('Content-Type', 'application/octet-stream')
        super().end_headers()
    
    def log_message(self, format, *args):
        """Override to provide better logging."""
        sys.stderr.write(f"[ASUS Repo Server] {format % args}\n")


def validate_repo_structure(repo_path: Path) -> bool:
    """Validate that the repository structure is correct."""
    required_paths = [
        'dists/noble/Release',
        'dists/noble/main/binary-arm64/Packages',
        'pool/main',
    ]
    
    for path in required_paths:
        full_path = repo_path / path
        if not full_path.exists():
            print(f"ERROR: Required path not found: {full_path}")
            return False
    
    print(f"✓ Repository structure validated at {repo_path}")
    return True


def start_server(repo_path: Path, port: int, bind_address: str = '0.0.0.0'):
    """Start the HTTP server to serve the repository."""
    os.chdir(repo_path)
    
    handler = ASUSRepoHTTPRequestHandler
    
    with socketserver.TCPServer((bind_address, port), handler) as httpd:
        print(f"\n{'='*60}")
        print(f"ASUS Ascent Repository Server")
        print(f"{'='*60}")
        print(f"Repository path: {repo_path}")
        print(f"Server address: http://{bind_address}:{port}")
        print(f"Repository URL: http://{bind_address}:{port}/")
        print(f"\nTo use in Dockerfile:")
        print(f'  RUN echo "deb [trusted=yes] http://{bind_address}:{port} ./" > /etc/apt/sources.list.d/asus-ascent.list')
        print(f"\nPress Ctrl+C to stop the server")
        print(f"{'='*60}\n")
        
        try:
            httpd.serve_forever()
        except KeyboardInterrupt:
            print("\n\nShutting down server...")
            httpd.shutdown()


def main():
    parser = argparse.ArgumentParser(
        description='Setup HTTP server for ASUS Ascent repository',
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  # Start server on default port 8000
  python scripts/setup_asus_repo_server.py --repo-path /path/to/ASUS-Ascent-GX10-OS-7.2.3-3-20251007074705-arm64

  # Start server on custom port
  python scripts/setup_asus_repo_server.py --repo-path /path/to/repo --port 9000

  # Start server bound to specific interface
  python scripts/setup_asus_repo_server.py --repo-path /path/to/repo --bind 127.0.0.1
        """
    )
    
    parser.add_argument(
        '--repo-path',
        type=Path,
        required=True,
        help='Path to the extracted ASUS Ascent ISO repository'
    )
    
    parser.add_argument(
        '--port',
        type=int,
        default=8000,
        help='Port to serve the repository on (default: 8000)'
    )
    
    parser.add_argument(
        '--bind',
        type=str,
        default='0.0.0.0',
        help='Address to bind to (default: 0.0.0.0 for all interfaces)'
    )
    
    args = parser.parse_args()
    
    # Validate repository path
    if not args.repo_path.exists():
        print(f"ERROR: Repository path does not exist: {args.repo_path}")
        sys.exit(1)
    
    if not args.repo_path.is_dir():
        print(f"ERROR: Repository path is not a directory: {args.repo_path}")
        sys.exit(1)
    
    # Validate repository structure
    if not validate_repo_structure(args.repo_path):
        print("\nERROR: Repository structure validation failed.")
        print("Please ensure you have extracted the complete ISO.")
        sys.exit(1)
    
    # Start the server
    start_server(args.repo_path, args.port, args.bind)


if __name__ == '__main__':
    main()

