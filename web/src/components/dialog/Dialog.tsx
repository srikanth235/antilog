// Follows stack/web/general/ui-components/dialog.md. Open state comes from the caller (the URL for places).
import { AlertDialog } from "@base-ui/react/alert-dialog";
import { Dialog as BaseDialog } from "@base-ui/react/dialog";
import { type ReactNode, useRef } from "react";
import { w } from "../../words";
import { Icon } from "../icon/Icon";
import styles from "./Dialog.module.css";

export function Dialog(props: {
  open: boolean;
  onClose: () => void;
  title: string;
  description?: string;
  size?: "narrow" | "settings";
  /** A wider popup than the narrow one, in pixels, for a form with rows. */
  width?: number;
  children: ReactNode;
}) {
  const settings = props.size === "settings";
  const popup = useRef<HTMLDivElement>(null);
  return (
    <BaseDialog.Root
      open={props.open}
      onOpenChange={(open) => {
        if (!open) props.onClose();
      }}
    >
      <BaseDialog.Portal>
        <BaseDialog.Backdrop className={styles.backdrop} />
        <BaseDialog.Popup
          ref={popup}
          className={settings ? `sd ${styles.settings}` : styles.popup}
          style={
            props.width === undefined
              ? undefined
              : { width: `min(${props.width}px, calc(100% - 32px))` }
          }
          // Focus goes to the first field, never to the close button (stack/web dialog: initial focus).
          initialFocus={() =>
            popup.current?.querySelector<HTMLElement>("[data-autofocus], input, textarea") ?? true
          }
        >
          <div className={settings ? "sd-top" : styles.head}>
            <BaseDialog.Title className="t-section">{props.title}</BaseDialog.Title>
            <BaseDialog.Close className="icon-btn" aria-label={w.close}>
              <Icon name="close" />
            </BaseDialog.Close>
          </div>
          {props.description === undefined ? null : (
            <BaseDialog.Description className={styles.description}>
              {props.description}
            </BaseDialog.Description>
          )}
          {props.children}
        </BaseDialog.Popup>
      </BaseDialog.Portal>
    </BaseDialog.Root>
  );
}

/** For what cannot be undone: the verb is the button. */
export function Confirm(props: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title: string;
  description: string;
  confirm: string;
  keep: string;
  onConfirm: () => void;
}) {
  return (
    <AlertDialog.Root open={props.open} onOpenChange={props.onOpenChange}>
      <AlertDialog.Portal>
        <AlertDialog.Backdrop className={styles.backdrop} />
        <AlertDialog.Popup className={styles.popup}>
          <AlertDialog.Title className="t-section">{props.title}</AlertDialog.Title>
          <AlertDialog.Description className={styles.description}>
            {props.description}
          </AlertDialog.Description>
          <div className={styles.actions}>
            <AlertDialog.Close className="btn">{props.keep}</AlertDialog.Close>
            <button type="button" className="btn primary" onClick={props.onConfirm}>
              {props.confirm}
            </button>
          </div>
        </AlertDialog.Popup>
      </AlertDialog.Portal>
    </AlertDialog.Root>
  );
}
