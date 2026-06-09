# Changelog

All notable changes to `openauth-axum` are documented in this file.

## [Unreleased]

## [0.1.1] - 2026-06-09

### Added

- `OpenAuthAxumExt::into_router_with` and `OpenAuthAxumExt::into_routes_with` as
  the adapter-specific mount entry points.

### Changed

- **Breaking:** Mount OpenAuth through `OpenAuthAxumExt` (`into_router`,
  `into_router_with`, `into_routes`, `into_routes_with`). Removed free functions
  `router`, `router_with_options`, `routes`, `routes_with_options`,
  `handle_ref`, and `handle_ref_with_options`.
- **Breaking:** Renamed `into_router_with_options` → `into_router_with` and
  `into_routes_with_options` → `into_routes_with`.

### Fixed

- Request base URL inference so request-derived `Host` values are not trusted
  origins, and disabled that inference by default.

## [0.0.6] - 2026-05-24

### Added

- Added explicit adapter options, request conversion, response handling, router,
  and error modules.
- Added HTTP contract, error contract, security, routing, and storage smoke
  coverage.
- Added parity coverage for body-consuming Tower middleware ordered before auth
  routes, locking the stable JSON error returned for drained request bodies.

### Changed

- Hardened Axum routing contracts and made adapter behavior easier to review
  through smaller modules.

## [0.0.5] - 2026-05-19

### Added

- Published the beta Axum integration release line.
