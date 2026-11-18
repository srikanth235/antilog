// A short list, one choice. Base UI's Select keeps a real button, a listbox and keyboard typeahead.
import { Select as Base } from "@base-ui/react/select";
import { Icon } from "../icon/Icon";
import styles from "./Select.module.css";

export interface Option<T extends string> {
  value: T;
  label: string;
}

export function Select<T extends string>(props: {
  label: string;
  value: T | null;
  options: readonly Option<T>[];
  onChange: (value: T) => void;
  disabled?: boolean;
}) {
  const current = props.options.find((option) => option.value === props.value);
  return (
    <Base.Root
      value={props.value}
      disabled={props.disabled === true}
      items={props.options.map((option) => ({ value: option.value, label: option.label }))}
      onValueChange={(value) => {
        if (value !== null) props.onChange(value as T);
      }}
    >
      <Base.Trigger className={styles.trigger} aria-label={props.label}>
        <span>{current?.label ?? ""}</span>
        <Icon name="chev" size="s" />
      </Base.Trigger>
      <Base.Portal>
        <Base.Positioner sideOffset={4} alignItemWithTrigger={false}>
          <Base.Popup className={styles.popup}>
            <Base.List>
              {props.options.map((option) => (
                <Base.Item key={option.value} value={option.value} className={styles.item}>
                  <Base.ItemText>{option.label}</Base.ItemText>
                </Base.Item>
              ))}
            </Base.List>
          </Base.Popup>
        </Base.Positioner>
      </Base.Portal>
    </Base.Root>
  );
}
