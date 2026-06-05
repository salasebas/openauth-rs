# openauth-fred — upstream parity

Full notes: [README.md § Upstream parity](./README.md#upstream-parity-better-auth-169).

Upstream: `@better-auth/redis-storage` @ 1.6.9. Sibling: `openauth-redis` (`redis-rs`).

Estimated server parity: **~95%** vs literal upstream adapter; **~98%** vs OpenAuth
contract (namespaces, validations). Product parity for session payloads depends on
`openauth-core`, not this crate.

```bash
cargo nextest run -p openauth-fred
```
