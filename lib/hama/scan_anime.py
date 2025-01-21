#!/usr/bin/env python3
import sys
import os
import argparse
from Contents.Code.common import *

def scan_folder(folder_path):
    """Scan a folder for anime episodes using HAMA's scanner."""
    try:
        # Initialize HAMA scanner
        scanner = Scanner()
        
        # Scan the folder
        results = scanner.Scan(folder_path)
        
        # Process and output results
        for item in results:
            if isinstance(item, Episode):
                if item.special:
                    print(f"Special:|{item.file}|{item.path}|{item.number}")
                else:
                    print(f"Episode:|{item.file}|{item.path}|{item.number}")
            
    except Exception as e:
        print(f"Error: {str(e)}", file=sys.stderr)
        sys.exit(1)

def main():
    parser = argparse.ArgumentParser(description='Scan anime folder using HAMA')
    parser.add_argument('--scan', required=True, help='Folder path to scan')
    args = parser.parse_args()
    
    if not os.path.exists(args.scan):
        print(f"Error: Path does not exist: {args.scan}", file=sys.stderr)
        sys.exit(1)
        
    scan_folder(args.scan)

if __name__ == '__main__':
    main() 