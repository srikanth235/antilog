// Follows stack/web/general/ui-components/dialog.md. Open state comes from the caller (the URL for places).
import { AlertDialog } from "@base-ui/react/alert-dialog";
import { Dialog as BaseDialog } from "@base-ui/react/dialog";
import type { ReactNode } from "react";
import { w } from "../../words";
import { Icon } from "../icon/Icon";
import styles from "./Dialog.module.css";

export function Dialog(props: {
  open: boolean;
  onClose: () => void;
  title: string;
  description?: string;
  size?: "narrow" | "settings";
  children: ReactNode;
}) {
  const settings = props.size === "settings";
  return (
    <BaseDialog.Root
      open={props.open}
      onOpenChange={(open) => {
        if (!open) props.onClose();
      }}
    >
      <BaseDialog.Portal>
        <BaseDialog.Backdrop className={styles.backdrop} />
        <BaseDialog.Popup className={settings ? `sd ${styles.settings}` : styles.popup}>
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
