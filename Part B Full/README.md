
# CYB225 Secure Coding – Part B (Rust Rewrite)

## Overview
Secure rewrite of the “Safe Backup” utility in Rust. Addresses input validation, safe file handling, and minimal logging. Demonstrated with backup, restore, and delete operations.

## Build & Run
```bash
# inside safe_backup_rust
cargo build --release
cd target/release

# demo
echo Hello World > test.txt
.\safe_backup.exe
# file name: test.txt
# command: backup

del test.txt
.\safe_backup.exe
# file name: test.txt
# command: restore

.\safe_backup.exe
# file name: test.txt
# command: delete
```

## Notes
- Restores from latest `test.txt.<timestamp>.bak` or `test.bak`.
- Validates filenames (no absolute paths/.. traversal).
- JSONL logging in `logfile.txt` with timestamp and user.
