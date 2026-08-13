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

## Included Function Families

- Capture
- Work
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

The next safe increment is to connect this registry to the command surface as discoverable commands, while still routing execution through existing UI flows and confirmation boundaries.
