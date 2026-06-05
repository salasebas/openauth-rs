# openauth-redis — upstream parity

Full notes: [README.md § Upstream parity](./README.md#upstream-parity-better-auth-169).

Upstream: `@better-auth/redis-storage` @ 1.6.9. Sibling: `openauth-fred`.

**KV adapter:** high parity. **Session payloads:** not portable to Better Auth without core migration. **Rate limit:** OpenAuth Lua extension; wire separately from secondary storage.

```bash
cargo nextest run -p openauth-redis
```
