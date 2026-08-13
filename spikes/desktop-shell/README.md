# M/OS Desktop Shell Spike

Disposable Tauri 2 experiment used to evaluate ADR-006. This is not M/OS product code.

## Covered capabilities

- single-instance Windows process;
- main window plus always-on-top Quick Capture;
- configurable global shortcut with rollback on registration failure;
- tray lifecycle;
- SQLite in WAL/FULL mode;
- transactional Capture and FTS5 projection;
- MSI and NSIS packaging;
- constrained Tauri capability and CSP;
- UI Automation semantics, keyboard flow, dark mode, reduced motion and forced colors.

## Commands

```powershell
npm install
npm run build
cd src-tauri
cargo test
cd ..
npm run tauri dev
npm run tauri build
```

The database is stored under the Tauri application data directory using the disposable identifier `com.codedbym.mos.spike`.

Results and the architectural scorecard are recorded in `docs/TECHNICAL-SPIKE-DESKTOP-SHELL.md`.
