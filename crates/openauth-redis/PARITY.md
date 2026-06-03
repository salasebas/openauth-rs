# openauth-redis — paridad servidor

Documentación: [`docs/parity/openauth-redis/README.md`](../../docs/parity/openauth-redis/README.md).

| Upstream | `@better-auth/redis-storage` @ 1.6.9 |
| Hermano | `openauth-fred` (mismo contrato; `list_keys`/`clear`; TTL 0 ≈ upstream) |

**Adaptador KV:** alta paridad (`get`/`set`/`delete`, `ttl=0`, `list_keys`/`clear`, `connect_with_options`). **Datos de sesión:** layout distinto en core ([08](../../docs/parity/openauth-redis/08-logical-keys-and-payloads.md)). **Rate limit:** Lua alineado con upstream en borde `>` ([11](../../docs/parity/openauth-redis/11-gap-closure-status.md)). **Tests:** 19 aquí; E2E sign-up en `openauth-fred`.

```bash
cargo nextest run -p openauth-redis
```
