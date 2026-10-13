type Props = { size?: number; className?: string };

/** A small vector mark for the relationship Chat UI manages: one coordinator
 * connected to several runnable surfaces. It stays crisp at every density and
 * deliberately avoids the terminal glyph used by the reference product. */
export function ChatUiLogo({ size = 28, className }: Props) {
  return (
    <svg
      className={className}
      width={size}
      height={size}
      viewBox="0 0 32 32"
      fill="none"
      aria-hidden="true"
    >
      <path
        d="M9 12.5 16 8l7 4.5M16 8v8M9 12.5v8L16 25l7-4.5v-8"
        stroke="currentColor"
        strokeWidth="1.8"
        strokeLinejoin="round"
      />
      <rect x="3.5" y="10" width="8" height="8" rx="2.7" fill="currentColor" />
      <rect x="12" y="4" width="8" height="8" rx="2.7" fill="#4389ED" />
      <rect x="20.5" y="16" width="8" height="8" rx="2.7" fill="currentColor" />
      <path
        d="M7.5 14h.01M16 8h.01M24.5 20h.01"
        stroke="#fff"
        strokeWidth="2.4"
        strokeLinecap="round"
      />
    </svg>
  );
}
