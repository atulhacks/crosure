/**
 * Crosure mark: an investigation path (three linked steps) closing on a
 * recording dot — a flight recorder for reverse engineering.
 */
export function Logo({ size = 18 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" aria-hidden>
      <path d="M5 5h6v6h6v6" stroke="var(--muted)" strokeWidth="1.6" strokeLinejoin="round" />
      <rect x="3" y="3" width="4" height="4" rx="1" fill="var(--foreground)" />
      <rect x="9" y="9" width="4" height="4" rx="1" fill="var(--foreground)" />
      <circle cx="17" cy="17" r="3.2" fill="var(--brand)" />
    </svg>
  );
}
