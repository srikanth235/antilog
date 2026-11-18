import { Tabs as Base } from "@base-ui/react/tabs";
import { Link } from "@tanstack/react-router";
import { w } from "../../words";

/** Each tab is a URL. Only Evaluations is in the first release; the rest say so. */
export function Tabs() {
  const later = [
    w.customize.skills,
    w.customize.connectors,
    w.customize.agents,
    w.customize.plugins,
  ];
  return (
    <Base.Root value="evaluations">
      <Base.List className="ctabs" aria-label={w.customize.tabs}>
        <Base.Tab
          value="evaluations"
          className="ptab on"
          nativeButton={false}
          render={<Link to="/customize/evaluations" />}
        >
          {w.customize.evaluations}
        </Base.Tab>
        {later.map((label) => (
          <Base.Tab
            key={label}
            value={label}
            className="ptab"
            disabled
            title={w.customize.laterTabs}
          >
            {label}
          </Base.Tab>
        ))}
      </Base.List>
    </Base.Root>
  );
}
