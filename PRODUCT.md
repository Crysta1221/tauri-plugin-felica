# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Users

Plugin authors and hardware testers at a desk with a USB PaSoRi reader. They need to confirm the Tauri FeliCa plugin talks to a real device and shows transit-card data. (Inferred from the example-app brief.)

## Product Purpose

`examples/` is a host app that exercises `tauri-plugin-felica`: list readers, start a scan, and display transit / e-money / Lite data from `ScanResult`. Ticket-gate simulation is explicitly deferred. Success is a sample that works on real hardware.

## Positioning

This is the plugin’s runnable proof, not a production gate UI. The mechanism is live USB reader I/O through the plugin API.

## Operating Context

Windows desktop Tauri v2 webview. Frontend: Vite+, React, TanStack Router, shadcn (`base-lyra`), Lucide. JS package manager: pnpm. Rust host in `examples/src-tauri`.

## Capabilities and Constraints

- Device select, then start scanning; show balance and related card data for every detected product on the card (transit, WAON, Edy, nanaco, QUICPay, Lite).
- A `/lab` route exists for explicit low-level reads. Gate-simulator UX is out of scope.
- UI primitives must come from shadcn only (no custom widget kit).
- Visual style is pinned: shadcn **base-lyra** (Base UI primitives via `@base-ui/react`, Lyra look).
- `examples/src` is feature-based.
- Quality gates: `pnpm lint` (`vp lint`) and `cargo check` in `src-tauri`.

## Brand Commitments

Name follows the plugin (`tauri-plugin-felica`). Binding UI constraint: shadcn base-lyra + Lucide. Copy may be Japanese for this lab sample.

## Evidence on Hand

No real card dumps in-repo. Empty, scanning, success, and error states must run against live hardware or honest empty/error copy. Do not fabricate balances or station names as if they were live reads. Do not invent Suica/ICOCA brand names from unkeyed data.

## Product Principles

- Prefer a working reader loop over unfinished gate chrome.
- Keep the plugin API as the only hardware boundary.
- Show device and scan state without inventing transit claims.
