/** RustAuth monogram geometry (matches `public/favicon/favicon.svg`). */

export const RUSTAUTH_MARK_VIEWBOX = "0 0 32 32";

export const RUSTAUTH_ACCENT = "#DE622A";

export const RUSTAUTH_MARK_PATH =
	"M7 6h13.3c4.2 0 6.9 2.45 6.9 6.2 0 2.62-1.34 4.54-3.62 5.54L28 26h-6.02l-3.88-7.18h-5.42V26H7V6Z";

export const RUSTAUTH_MARK_COUNTER_PATH =
	"M12.68 10.74v3.68h7.04c1.38 0 2.22-.72 2.22-1.84 0-1.1-.84-1.84-2.22-1.84h-7.04Z";

export const RUSTAUTH_PROMPT_PATH =
	"M4.9 14.1 8.28 17.5 4.9 20.9";

type MarkColors = {
	fill: string;
	cutout: string;
	includeAccent?: boolean;
};

function markShapes({ fill, cutout, includeAccent = true }: MarkColors): string {
	const accent = includeAccent
		? `<circle cx="25.15" cy="7.45" r="2.35" fill="${RUSTAUTH_ACCENT}"/>`
		: "";

	return `
<path d="${RUSTAUTH_MARK_PATH}" fill="${fill}"/>
<path d="${RUSTAUTH_MARK_COUNTER_PATH}" fill="${cutout}"/>
<path d="${RUSTAUTH_PROMPT_PATH}" stroke="${fill}" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round"/>
${accent}`.trim();
}

export function rustauthMarkSvg({
	fill,
	cutout,
	includeAccent = true,
	viewBox = RUSTAUTH_MARK_VIEWBOX,
}: MarkColors & { viewBox?: string }): string {
	return `<svg viewBox="${viewBox}" fill="none" xmlns="http://www.w3.org/2000/svg">${markShapes({ fill, cutout, includeAccent })}</svg>`;
}

function scaledMarkTransform(scale: number, offsetX: number, offsetY: number): string {
	return `translate(${offsetX} ${offsetY}) scale(${scale})`;
}

export function rustauthLogoSquareSvg({
	background,
	fill,
	cutout,
}: {
	background: string;
	fill: string;
	cutout: string;
}): string {
	const scale = 10;
	const offset = (500 - 32 * scale) / 2;

	return `<svg width="500" height="500" viewBox="0 0 500 500" fill="none" xmlns="http://www.w3.org/2000/svg">
<rect width="500" height="500" fill="${background}"/>
<g transform="${scaledMarkTransform(scale, offset, offset)}">
${markShapes({ fill, cutout })}
</g>
</svg>`;
}

export function rustauthWordmarkSvg({
	background,
	fill,
	cutout,
}: {
	background: string;
	fill: string;
	cutout: string;
}): string {
	return `<svg width="1024" height="256" viewBox="0 0 1024 256" fill="none" xmlns="http://www.w3.org/2000/svg">
<rect width="1024" height="256" fill="${background}"/>
<g transform="${scaledMarkTransform(4.25, 72, 60)}">
${markShapes({ fill, cutout })}
</g>
<text x="240" y="154" fill="${fill}" font-family="'Geist Mono', 'SFMono-Regular', Consolas, monospace" font-size="82" font-weight="500" letter-spacing="5">RUSTAUTH.</text>
</svg>`;
}

export const logoAssets = {
	darkSvg: rustauthLogoSquareSvg({
		background: "#141413",
		fill: "white",
		cutout: "#141413",
	}),
	whiteSvg: rustauthLogoSquareSvg({
		background: "#fafafa",
		fill: "#141413",
		cutout: "#fafafa",
	}),
	darkWordmark: rustauthWordmarkSvg({
		background: "#141413",
		fill: "white",
		cutout: "#141413",
	}),
	whiteWordmark: rustauthWordmarkSvg({
		background: "#fafafa",
		fill: "#141413",
		cutout: "#fafafa",
	}),
};
