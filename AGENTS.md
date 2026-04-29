# AGENTS.md

## Scope

This file applies to the `E:\code\Homogram\features\home\src\main\native\homogrape` submodule.

## Submodule Summary

- `homogrape` is the Rust native layer for Homogram.
- It builds the N-API/OpenHarmony shared library consumed by the ArkTS app as `libhomogrape.so`.
- The app imports that library from the `features/home` module.

## Important Paths

- Rust sources: `src/`
- Telegram config: `src/tg/config.rs`
- Template for config: `src/tg/config.rs.template`
- Build helper: `xtask/src/main.rs`
- Output staging directory after `ohrs build`: `dist/arm64-v8a`
- Consumer copy target in the parent repo: `E:\code\Homogram\features\home\libs\arm64-v8a`

## Local Build Contract

Before the parent HarmonyOS app is packaged, this submodule must build and copy its `.so` into the parent repo.

Run from this directory:

```powershell
cargo xtask dist ../../../../libs/arm64-v8a/
```

What that command does:

- invokes `ohrs build --arch=aarch`
- reads built libraries from `dist/arm64-v8a`
- copies them into the parent repo's `features/home/libs/arm64-v8a`

Prerequisites:

- `OHOS_NDK_HOME` is already set in the shell environment and points to the HarmonyOS NDK root
- `ohrs` is on `PATH`
- the Rust toolchain is installed

Typical local invocation:

```powershell
$env:RUSTFLAGS='-Awarnings'
cargo xtask dist ../../../../libs/arm64-v8a/
```

## Editing Rules

- Keep Rust changes scoped. This submodule is consumed by the parent app and should not be rewritten casually.
- Preserve the exported API shape expected by ArkTS imports such as `login`, `verifyCode`, `password`, `signOut`, `run`, and `reconnect` unless the parent app is updated in the same change.
- When touching Telegram config handling, keep `src/tg/config.rs` and `src/tg/config.rs.template` aligned.
- Do not introduce too much defensive logic. Instead, think in-depth and design carefully to ensure the code quality and correctness in a structural way.
## Parent App Handoff

After rebuilding this submodule, the parent repo should package the `phone` HAP with Hvigor. The parent repo's root `AGENTS.md` contains the full HAP/signing workflow.
