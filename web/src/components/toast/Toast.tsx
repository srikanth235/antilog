// A short confirmation after an action (stack/web/general/ui-components/persistent-toast-notifications.md).
import { Toast } from "@base-ui/react/toast";
import type { ReactNode } from "react";
import styles from "./Toast.module.css";

function List() {
  const { toasts } = Toast.useToastManager();
  return (
    <Toast.Portal>
      <Toast.Viewport className={styles.viewport}>
        {toasts.map((toast) => (
          <Toast.Root key={toast.id} toast={toast} className={styles.toast}>
            <Toast.Title className={styles.title} />
            <Toast.Description className={styles.description} />
          </Toast.Root>
        ))}
      </Toast.Viewport>
    </Toast.Portal>
  );
}

export function ToastProvider({ children }: { children: ReactNode }) {
  return (
    <Toast.Provider timeout={4000}>
      {children}
      <List />
    </Toast.Provider>
  );
}

export function useToast() {
  const manager = Toast.useToastManager();
  return (title: string, description?: string) =>
    manager.add(description === undefined ? { title } : { title, description });
}
