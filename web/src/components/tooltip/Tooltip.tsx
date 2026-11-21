import { Tooltip as Base } from "@base-ui/react/tooltip";
import type { ReactElement } from "react";
import styles from "./Tooltip.module.css";

/** A hint for an icon-only control. The control keeps its own aria-label. */
export function Tip({
  label,
  children,
}: {
  label: string;
  children: ReactElement<Record<string, unknown>>;
}) {
  return (
    <Base.Root>
      <Base.Trigger render={children} />
      <Base.Portal>
        <Base.Positioner sideOffset={6}>
          <Base.Popup className={styles.popup}>{label}</Base.Popup>
        </Base.Positioner>
      </Base.Portal>
    </Base.Root>
  );
}

export const TipProvider = Base.Provider;
