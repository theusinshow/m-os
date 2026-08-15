import { cn } from "@/lib/utils";

/**
 * Coded by M triangle glyph. Used as the product mark, nav markers and
 * geometric ornaments. Outline + inner triangle echo the brand structure.
 */
export function TriangleMark({
  className,
  size = 24,
  variant = "outline",
}: {
  className?: string;
  size?: number;
  variant?: "outline" | "solid" | "nested";
}) {
  return (
    <svg
      aria-hidden="true"
      className={cn("block", className)}
      fill="none"
      height={size}
      viewBox="0 0 24 24"
      width={size}
    >
      {variant === "solid" ? (
        <path d="M12 3 22 20.5H2L12 3Z" fill="currentColor" />
      ) : (
        <path
          d="M12 3 22 20.5H2L12 3Z"
          stroke="currentColor"
          strokeLinejoin="round"
          strokeWidth="1.5"
        />
      )}
      {variant === "nested" ? (
        <path d="M12 10 17 19.5H7L12 10Z" fill="currentColor" opacity="0.9" />
      ) : null}
    </svg>
  );
}
