interface Props {
  type: "desktop" | "mobile";
  children: React.ReactNode;
  className?: string;
}

export function DeviceFrame({ type, children, className = "" }: Props) {
  if (type === "desktop") {
    return (
      <div className={`border border-line bg-surface overflow-hidden ${className}`}>
        {/* Browser chrome */}
        <div className="flex items-center gap-1.5 px-3 py-2 border-b border-line bg-base shrink-0">
          <span className="w-2 h-2 rounded-full bg-surface-2" />
          <span className="w-2 h-2 rounded-full bg-surface-2" />
          <span className="w-2 h-2 rounded-full bg-surface-2" />
        </div>
        {children}
      </div>
    );
  }

  return (
    <div className={`border border-line bg-surface overflow-hidden ${className}`}>
      {/* Status bar */}
      <div className="h-2 bg-base" />
      {children}
      {/* Home indicator */}
      <div className="h-3 bg-base flex items-center justify-center">
        <span className="w-10 h-px bg-surface-2" />
      </div>
    </div>
  );
}
