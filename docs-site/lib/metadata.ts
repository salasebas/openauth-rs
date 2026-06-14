import type { Metadata } from "next";

export function createMetadata(override: Metadata): Metadata {
	return {
		...override,
		metadataBase: baseUrl,
		openGraph: {
			title: override.title ?? undefined,
			description: override.description ?? undefined,
			url: "https://rustauth.dev",
			images: "/og.png",
			siteName: "RustAuth",
			...override.openGraph,
		},
		twitter: {
			card: "summary_large_image",
			title: override.title ?? undefined,
			description: override.description ?? undefined,
			images: "/og.png",
			...override.twitter,
		},
		icons: {
			icon: [
				{ url: "/favicon/favicon.svg", type: "image/svg+xml" },
				{ url: "/favicon/favicon.ico", sizes: "any" },
				{
					url: "/favicon/favicon-32x32.png",
					sizes: "32x32",
					type: "image/png",
				},
				{
					url: "/favicon/favicon-16x16.png",
					sizes: "16x16",
					type: "image/png",
				},
			],
			apple: "/favicon/apple-touch-icon.png",
		},
	};
}

function resolveBaseUrl(): URL {
	if (process.env.NODE_ENV === "development") {
		return new URL("http://localhost:3000");
	}

	if (process.env.NEXT_PUBLIC_URL) {
		return new URL(process.env.NEXT_PUBLIC_URL);
	}

	if (process.env.VERCEL_PROJECT_PRODUCTION_URL || process.env.VERCEL_URL) {
		return new URL(
			`https://${process.env.VERCEL_PROJECT_PRODUCTION_URL || process.env.VERCEL_URL}`,
		);
	}

	return new URL("https://rustauth.dev");
}

export const baseUrl = resolveBaseUrl();
