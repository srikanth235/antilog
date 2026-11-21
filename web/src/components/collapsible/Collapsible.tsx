import { Collapsible as Base } from "@base-ui/react/collapsible";
import type { ReactNode } from "react";
import { Icon } from "../icon/Icon";

/** A sidebar project group that folds. */
export function Group(props: {
  title: string;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  children: ReactNode;
}) {
  return (
    <Base.Root className="pg" open={props.open} onOpenChange={props.onOpenChange}>
      <Base.Trigger className="ph" aria-expanded={props.open}>
        <Icon name="chev" size="s" />
        <span className="t">{props.title}</span>
      </Base.Trigger>
      <Base.Panel keepMounted={false}>{props.children}</Base.Panel>
    </Base.Root>
  );
}
