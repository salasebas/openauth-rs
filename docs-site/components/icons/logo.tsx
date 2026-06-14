import type { SVGProps } from "react";
import { cn } from "@/lib/utils";
import {
	RUSTAUTH_ACCENT,
	RUSTAUTH_MARK_COUNTER_PATH,
	RUSTAUTH_MARK_PATH,
	RUSTAUTH_MARK_VIEWBOX,
	RUSTAUTH_PROMPT_PATH,
} from "@/lib/branding/rustauth-mark";

type LogoProps = {
	className?: string;
	showAccent?: boolean;
};

export const Logo = ({ className, showAccent = true }: LogoProps) => {
	return (
		<svg
			className={className || "h-5 w-5"}
			viewBox={RUSTAUTH_MARK_VIEWBOX}
			fill="none"
			xmlns="http://www.w3.org/2000/svg"
			aria-hidden="true"
		>
			<path d={RUSTAUTH_MARK_PATH} className="fill-foreground" />
			<path d={RUSTAUTH_MARK_COUNTER_PATH} className="fill-background" />
			<path
				d={RUSTAUTH_PROMPT_PATH}
				className="stroke-foreground"
				strokeWidth="2.4"
				strokeLinecap="round"
				strokeLinejoin="round"
			/>
			{showAccent ? (
				<circle cx="25.15" cy="7.45" r="2.35" fill={RUSTAUTH_ACCENT} />
			) : null}
		</svg>
	);
};

export const LogoMark = ({
	className,
	showAccent = true,
	...props
}: SVGProps<SVGSVGElement> & { showAccent?: boolean }) => {
	return (
		<svg
			{...props}
			viewBox={RUSTAUTH_MARK_VIEWBOX}
			fill="none"
			xmlns="http://www.w3.org/2000/svg"
			className={cn("h-5 w-5", className)}
			aria-hidden="true"
		>
			<path d={RUSTAUTH_MARK_PATH} className="fill-foreground" />
			<path d={RUSTAUTH_MARK_COUNTER_PATH} className="fill-background" />
			<path
				d={RUSTAUTH_PROMPT_PATH}
				className="stroke-foreground"
				strokeWidth="2.4"
				strokeLinecap="round"
				strokeLinejoin="round"
			/>
			{showAccent ? (
				<circle cx="25.15" cy="7.45" r="2.35" fill={RUSTAUTH_ACCENT} />
			) : null}
		</svg>
	);
};
