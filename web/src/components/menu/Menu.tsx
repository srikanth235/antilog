// Follows stack/web/general/ui-components/menu.md.
import { Menu as BaseMenu } from "@base-ui/react/menu";
import type { ReactNode } from "react";

export function Menu(props: {
  label: string;
  trigger: ReactNode;
  className?: string;
  children: ReactNode;
}) {
  return (
    <BaseMenu.Root>
      <BaseMenu.Trigger className={props.className ?? "icon-btn"} aria-label={props.label}>
        {props.trigger}
      </BaseMenu.Trigger>
      <BaseMenu.Portal>
        <BaseMenu.Positioner sideOffset={6} align="end">
          <BaseMenu.Popup className="menu">{props.children}</BaseMenu.Popup>
        </BaseMenu.Positioner>
      </BaseMenu.Portal>
    </BaseMenu.Root>
  );
}

export function MenuItem(props: { onSelect: () => void; disabled?: boolean; children: ReactNode }) {
  return (
    <BaseMenu.Item className="mi" disabled={props.disabled === true} onClick={props.onSelect}>
      {props.children}
    </BaseMenu.Item>
  );
}
