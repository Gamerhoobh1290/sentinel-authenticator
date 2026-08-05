import { type SVGProps } from "react";

/**
 * Sentinel shield logo.
 * Designed to be readable at 16px and crisp at 256px.
 * Uses currentColor for theming.
 */
export function SentinelLogo({
  size = 24,
  ...rest
}: SVGProps<SVGSVGElement> & { size?: number }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 32 32"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      aria-hidden="true"
      {...rest}
    >
      <defs>
        <linearGradient
          id="sentinel-shield-grad"
          x1="16"
          y1="2"
          x2="16"
          y2="30"
          gradientUnits="userSpaceOnUse"
        >
          <stop stopColor="currentColor" stopOpacity="0.95" />
          <stop offset="1" stopColor="currentColor" stopOpacity="0.7" />
        </linearGradient>
      </defs>
      {/* Shield outline */}
      <path
        d="M16 2.5L4 6.5V14C4 21.5 8.8 27.7 16 29.5C23.2 27.7 28 21.5 28 14V6.5L16 2.5Z"
        fill="url(#sentinel-shield-grad)"
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinejoin="round"
      />
      {/* Inner keyhole / lock symbol */}
      <circle cx="16" cy="13.5" r="2.8" fill="white" fillOpacity="0.95" />
      <rect
        x="14.8"
        y="15"
        width="2.4"
        height="6"
        rx="1.2"
        fill="white"
        fillOpacity="0.95"
      />
    </svg>
  );
}
