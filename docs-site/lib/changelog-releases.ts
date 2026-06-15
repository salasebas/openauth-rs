export interface ChangelogRelease {
	tag: string;
	title: string;
	content: string;
	date: string;
	url: string;
}

const UNRELEASED_URL =
	"https://github.com/salasebas/rustauth/blob/main/CHANGELOG.md#unreleased";
const V030_URL =
	"https://github.com/salasebas/rustauth/blob/main/CHANGELOG.md#030---2026-06-15";
const V020_URL =
	"https://github.com/salasebas/rustauth/blob/main/CHANGELOG.md#020---2026-06-14";

export const changelogReleases: ChangelogRelease[] = [
	{
		tag: "Unreleased",
		title: "Unreleased",
		date: "2026-06-15",
		url: UNRELEASED_URL,
		content: `Changes on \`main\` not yet published to crates.io.

[Full changelog →](${UNRELEASED_URL})`,
	},
	{
		tag: "v0.3.0",
		title: "0.3.0 — Actix Web, Diesel adapters, CLI breaking changes",
		date: "2026-06-15",
		url: V030_URL,
		content: `Adds Actix Web integration, Diesel storage adapters, and CLI breaking changes.

### Added

- **\`rustauth-actix-web\`** — Actix Web adapter (\`RustAuthActixWebExt\`), parity tests, docs-site guide, and \`examples/actix-web-minimal\`.
- **\`rustauth-diesel\`** — async Diesel adapters for Postgres and MySQL (\`diesel-postgres\`, \`diesel-mysql\` features).
- **CLI** — \`rustauth init --framework actix-web\`, Actix workspace detection, and telemetry support.

### Changed

- **Breaking:** \`rustauth init\` requires \`--framework axum\` or \`--framework actix-web\` (no implicit default).
- **Breaking:** \`database.adapter\` is required in \`rustauth.toml\` and for \`rustauth init\` (no implicit \`sqlx\` default).

[Full release notes →](${V030_URL})`,
	},
	{
		tag: "v0.2.0",
		title: "0.2.0 — initial public working release",
		date: "2026-06-14",
		url: V020_URL,
		content: `First public release of **RustAuth** under the \`rustauth\` / \`rustauth-*\` crate namespace.

### Added

- Core auth server (\`rustauth\`, \`rustauth-core\`): sessions, cookies, rate limits, opt-in email/password, plugins, hooks, and Better Auth–shaped HTTP JSON.
- Axum integration (\`rustauth-axum\`), CLI (\`rustauth-cli\`), and \`rustauth.toml\` migration workflow.
- Official plugins (\`rustauth-plugins\`): admin, organization, JWT, API keys, magic link, email OTP, two-factor, SIWE, CAPTCHA, and more.
- Enterprise identity: OAuth client (\`rustauth-oauth\`), social providers, OAuth/OIDC provider, OIDC RP, SAML, SSO, SCIM, passkeys, Stripe, i18n, telemetry.
- Storage adapters: SQLx, tokio-postgres, deadpool-postgres, Redis, Fred.

[Full release notes →](${V020_URL})`,
	},
];

export const EXPANDABLE_LINE_THRESHOLD = 15;

export function isExpandableRelease(content: string): boolean {
	const lineCount = content
		.split("\n")
		.filter((line) => line.trim().length > 0).length;
	return lineCount > EXPANDABLE_LINE_THRESHOLD;
}

export function formatReleaseDate(isoDate: string): string {
	return new Date(isoDate).toLocaleDateString("en-US", {
		year: "numeric",
		month: "short",
		day: "numeric",
	});
}
