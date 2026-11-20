import { useState } from "react";
import type { Schemas } from "../../api/client";
import { useMember, useModelKeys, useRemoveModelKey, useSetModelKey } from "../../api/queries";
import { Confirm } from "../../components/dialog/Dialog";
import { Icon } from "../../components/icon/Icon";
import { Failed, Loading, reason } from "../../components/state/State";
import { useToast } from "../../components/toast/Toast";
import { isOwner } from "../../lib/domain";
import { date } from "../../lib/format";
import { w } from "../../words";

type Provider = Schemas["ModelProvider"];

const PROVIDERS: readonly Provider[] = ["anthropic", "openai"];

/** The organization's own keys for the model providers. A key goes in and is never shown again:
 * the list says which providers have one and when it was set. */
export function ModelKeys() {
  const keys = useModelKeys();
  const set = useSetModelKey();
  const remove = useRemoveModelKey();
  const toast = useToast();
  const me = useMember();
  const owner = isOwner(me?.role);
  const t = w.settings.modelKeys;
  const [adding, setAdding] = useState<Provider | null>(null);
  const [value, setValue] = useState("");
  const [removing, setRemoving] = useState<Provider | null>(null);

  const close = () => {
    setAdding(null);
    setValue("");
  };

  return (
    <div className="sd-sec">
      <h3 className="t-section">{w.settings.sections["model-keys"]}</h3>
      <p className="sd-lead">{t.lead}</p>
      <h4 className="sd-gh">{t.providers}</h4>
      {keys.isPending ? (
        <Loading rows={2} />
      ) : keys.isError ? (
        <Failed error={keys.error} retry={() => void keys.refetch()} />
      ) : (
        PROVIDERS.map((provider) => {
          const found = keys.data.find((key) => key.provider === provider);
          const label = t[provider].name;
          if (adding === provider) {
            return (
              <form
                key={provider}
                className="stg"
                style={{ alignItems: "flex-start" }}
                onSubmit={(event) => {
                  event.preventDefault();
                  set.mutate(
                    { provider, key: value.trim() },
                    {
                      onSuccess: () => {
                        close();
                        toast(t.saved);
                      },
                    },
                  );
                }}
              >
                <div className="tx" style={{ flex: 1 }}>
                  <span className="t-item rw g8">
                    <span className={`lg lg-${provider}`} role="img" aria-label={label} />
                    {t.formTitle(label)}
                  </span>
                  <span className="dd">{t.formHint(label)}</span>
                  <label className="insec" style={{ marginTop: 8 }}>
                    <Icon name="lock" />
                    <input
                      className="in mono"
                      type="password"
                      aria-label={t.keyLabel(label)}
                      value={value}
                      autoComplete="off"
                      spellCheck={false}
                      onChange={(event) => setValue(event.target.value)}
                    />
                  </label>
                  {set.isError ? (
                    <p className="error-line" role="alert">
                      {reason(set.error)}
                    </p>
                  ) : null}
                  <div className="rw g8" style={{ marginTop: 12 }}>
                    <button
                      type="submit"
                      className="btn primary sm"
                      disabled={set.isPending || value.trim() === ""}
                    >
                      {t.save}
                    </button>
                    <button type="button" className="btn sm" onClick={close}>
                      {t.cancel}
                    </button>
                  </div>
                </div>
              </form>
            );
          }
          return (
            <div key={provider} className="stg">
              <div className="tx">
                <span className="t-item rw g8">
                  <span className={`lg lg-${provider}`} role="img" aria-label={label} />
                  {label}
                </span>
                <span className="dd">{t[provider].hint}</span>
              </div>
              <div className="ct">
                {found === undefined ? (
                  <>
                    <span className="val mut">{t.none}</span>
                    <button
                      type="button"
                      className="btn primary sm"
                      disabled={!owner}
                      aria-label={`${t.add} ${label}`}
                      onClick={() => {
                        set.reset();
                        setAdding(provider);
                      }}
                    >
                      {t.add}
                    </button>
                  </>
                ) : (
                  <>
                    <span className="stc">
                      <Icon name="check" />
                      <b>{t.set}</b>
                    </span>
                    <span className="val xs mut">{date(found.setAt)}</span>
                    <button
                      type="button"
                      className="btn sm"
                      disabled={!owner}
                      aria-label={`${t.replace} ${label}`}
                      onClick={() => {
                        set.reset();
                        setAdding(provider);
                      }}
                    >
                      {t.replace}
                    </button>
                    <button
                      type="button"
                      className="btn sm"
                      disabled={!owner}
                      aria-label={`${t.remove} ${label}`}
                      onClick={() => setRemoving(provider)}
                    >
                      {t.remove}
                    </button>
                  </>
                )}
              </div>
            </div>
          );
        })
      )}
      <p className="ap-sec" style={{ marginTop: 12 }}>
        <Icon name="lock" />
        {t.ownersOnly}
      </p>
      <Confirm
        open={removing !== null}
        onOpenChange={(open) => {
          if (!open) setRemoving(null);
        }}
        title={removing === null ? "" : t.confirmTitle(t[removing].name)}
        description={removing === null ? "" : t.confirm(t[removing].name)}
        confirm={t.remove}
        keep={t.keepIt}
        onConfirm={() => {
          if (removing !== null)
            remove.mutate(removing, {
              onSuccess: () => {
                setRemoving(null);
                toast(t.removed);
              },
            });
        }}
      />
    </div>
  );
}
