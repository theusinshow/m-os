# Technical Foundation v0.2 - Functions

## Decision

M/OS now has a small local Function Registry for capabilities that already exist in the product.

This is a foundation layer, not the automation layer. A Function describes a stable internal capability with:

- stable id
- human name
- description
- category
- risk level
- confirmation policy

## Current Scope

The registry is static and compiled into `mos-core`.

It can be listed and searched by the desktop shell through Tauri commands.

The Settings screen exposes the registry so the product can inspect its current operational surface without introducing a heavy Functions UI.

The Command surface can discover Functions by text search. Selection is fully operable with keyboard through Arrow Up, Arrow Down and Enter.

## Intent Routing

Low-risk Functions use an explicit, fail-closed routing table in the desktop renderer.

- creation Functions open the existing form for Capture, Task, Project, Workspace, App or Resource;
- Functions that need an existing entity open the relevant context instead of guessing a target;
- `capture.quick_open` opens the existing Quick Capture window;
- `system.update_check` starts the existing update check because selecting the Function is an explicit low-risk intent;
- medium-risk, high-risk and unknown Functions open the registry without executing behavior;
- restore, export, backup, update installation and App launch keep their existing selection and confirmation boundaries.

The routing table belongs to the renderer because it describes UI destinations. The Core keeps stable capability definitions and does not gain knowledge of pages, forms or focus behavior.

## Included Function Families

- Capture
- Work
- Memory
- App Registry
- Data and portability
- System updates

## Explicitly Out Of Scope

- Hermes
- natural language execution
- app plugins
- user-defined functions
- automation chains
- scheduled jobs
- cross-app scripting
- remote execution
- cloud sync

## Product Reasoning

Functions must grow from real M/OS behavior, not from imagined automation.

This keeps the core independent from Hermes while giving future intelligence and command surfaces a stable vocabulary. Later, Hermes can ask for or execute the same functions the UI already uses, with clear risk and confirmation metadata.

## Backup and Export

The Function Registry is not user data. It is not exported as part of JSON export and is not stored in backups.

Backups still cover local user data. Function definitions are restored by the installed application version.

## Next Step

Dogfood the routed Functions and observe which actions need structured arguments before introducing parameter collection or broader execution. Natural-language interpretation, automation chains and Hermes remain out of scope.
