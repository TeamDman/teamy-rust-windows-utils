# v0.2.0 — 2025-09-24

- Add `EasyPCWSTR` implementation for &Path
- Add `fn watch_file_content`
- Add `HandleReadExt::try_read_exact(offset, buf)` implemented for `HANDLE`
- Add `enable_backup_privileges()` to enable `SE_BACKUP_NAME`, `SE_RESTORE_NAME`, and `SE_SECURITY_NAME` for the current process (useful for raw disk reads).
- Dependencies: added `crossbeam-channel` and `uom`.

# v0.1.0

- Copied stuff from various repos
    - https://github.com/TeamDman/youre-muted-btw.git
    - https://github.com/TeamDman/teamy-mft.git
    - https://github.com/TeamDman/teamy-rust-cli.git