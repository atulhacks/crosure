/** Crosure mark: a ring with step nodes. */
export function Logo({ size = 22 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 32 32" aria-hidden>
      <circle
        cx="16"
        cy="16"
        r="10"
        fill="none"
        stroke="#22d3ee"
        strokeWidth="2.5"
        strokeDasharray="48 15"
      />
      <circle cx="16" cy="16" r="2.6" fill="#a78bfa" />
      <circle cx="24" cy="16" r="2.2" fill="#22d3ee" />
      <circle cx="11" cy="11" r="2" fill="#a78bfa" />
    </svg>
  );
}
