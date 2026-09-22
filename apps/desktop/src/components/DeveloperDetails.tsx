import type { ReactNode } from "react";

export function DeveloperDetails({
  children,
  label = "技術情報",
}: {
  children: ReactNode;
  label?: string;
}) {
  return (
    <details className="developer-details">
      <summary>{label}</summary>
      <div className="details-body">{children}</div>
    </details>
  );
}
