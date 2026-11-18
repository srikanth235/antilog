import { Link, useNavigate } from "@tanstack/react-router";
import { Dialog } from "../../components/dialog/Dialog";
import { Glyph, Icon } from "../../components/icon/Icon";
import { w } from "../../words";
import {
  Appearance,
  Approvals,
  Audit,
  Budgets,
  General,
  Members,
  Profile,
  Tokens,
} from "./Sections";

export type Section =
  | "profile"
  | "appearance"
  | "tokens"
  | "general"
  | "members"
  | "approvals"
  | "budgets"
  | "audit";

export const SECTIONS: readonly Section[] = [
  "profile",
  "appearance",
  "tokens",
  "general",
  "members",
  "approvals",
  "budgets",
  "audit",
];

const RAIL: { group: "you" | "organization"; items: { id: Section; icon: React.ReactNode }[] }[] = [
  {
    group: "you",
    items: [
      { id: "profile", icon: <Glyph name="user" /> },
      { id: "appearance", icon: <Icon name="sun" /> },
      { id: "tokens", icon: <Icon name="key" /> },
    ],
  },
  {
    group: "organization",
    items: [
      { id: "general", icon: <Icon name="org" /> },
      { id: "members", icon: <Icon name="users" /> },
      { id: "approvals", icon: <Icon name="verify" /> },
      { id: "budgets", icon: <Glyph name="gauge" /> },
      { id: "audit", icon: <Glyph name="hist" /> },
    ],
  },
];

const BODY: Record<Section, () => React.ReactNode> = {
  profile: () => <Profile />,
  appearance: () => <Appearance />,
  tokens: () => <Tokens />,
  general: () => <General />,
  members: () => <Members />,
  approvals: () => <Approvals />,
  budgets: () => <Budgets />,
  audit: () => <Audit />,
};

/** Settings is a dialog whose place is in the URL: /settings/<section>. */
export function SettingsDialog({ section }: { section: Section }) {
  const navigate = useNavigate();
  return (
    <Dialog
      open
      onClose={() => void navigate({ to: "/" })}
      title={w.settings.title}
      size="settings"
    >
      <div className="sd-body">
        <div className="sd-rail">
          <nav className="sd-groups" aria-label={w.settings.rail}>
            {RAIL.map((group) => (
              <div key={group.group} className="sd-grp">
                <span className="t-label">{w.settings.groups[group.group]}</span>
                {group.items.map((item) => (
                  <Link
                    key={item.id}
                    className="sd-nav"
                    to="/settings/$section"
                    params={{ section: item.id }}
                    aria-current={item.id === section ? "page" : undefined}
                    data-item={item.id}
                  >
                    {item.icon}
                    <span className="grow ell">{w.settings.sections[item.id]}</span>
                  </Link>
                ))}
              </div>
            ))}
          </nav>
        </div>
        <div className="sd-main">{BODY[section]()}</div>
      </div>
    </Dialog>
  );
}
