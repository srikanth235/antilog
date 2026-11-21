import { Tabs as Base } from "@base-ui/react/tabs";
import { Link } from "@tanstack/react-router";
import { w } from "../../words";

export type Tab = "skills" | "connectors" | "plugins";

/** Each tab is a URL. Agents is not in the first release and says so. */
export function Tabs({ active }: { active: Tab }) {
  const tab = (value: Tab, to: string, label: string) => (
    <Base.Tab
      key={value}
      value={value}
      className={value === active ? "ptab on" : "ptab"}
      nativeButton={false}
      render={<Link to={to} />}
    >
      {label}
    </Base.Tab>
  );
  return (
    <Base.Root value={active}>
      <Base.List className="ctabs" aria-label={w.customize.tabs}>
        {tab("skills", "/customize/skills", w.customize.skills)}
        {tab("connectors", "/customize/connectors", w.customize.connectors)}
        <Base.Tab value="agents" className="ptab" disabled title={w.customize.laterTabs}>
          {w.customize.agents}
        </Base.Tab>
        {tab("plugins", "/customize/plugins", w.customize.plugins)}
      </Base.List>
    </Base.Root>
  );
}
