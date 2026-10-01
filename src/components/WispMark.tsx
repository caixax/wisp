interface WispMarkProps {
  className?: string;
}

/** The Wisp flame, inline so it inherits the page's colours and scales. */
export function WispMark({ className }: WispMarkProps) {
  return (
    <svg className={className} viewBox="0 0 32 32" aria-hidden="true">
      <defs>
        <linearGradient id="wisp-mark" x1="0" y1="1" x2="0.3" y2="0">
          <stop offset="0%" stopColor="var(--accent-strong)" />
          <stop offset="55%" stopColor="var(--accent)" />
          <stop offset="100%" stopColor="#ecfdff" />
        </linearGradient>
      </defs>
      <path
        d="M16 3c.6 5.4 5.5 8.4 5.5 14a5.5 5.5 0 0 1-11 0c0-3.3 2.2-5.3 3.4-8.2.7 1.7 1.6 2.6 2.1 4C16.9 9.4 15.6 6.4 16 3Z"
        fill="url(#wisp-mark)"
      />
    </svg>
  );
}
