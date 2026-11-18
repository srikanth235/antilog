import type { Schemas } from "../../api/client";
import { w } from "../../words";
import type { Option } from "./Select";

export const ROLE_OPTIONS: readonly Option<Schemas["Role"]>[] = [
  { value: "Viewer", label: w.roles.Viewer },
  { value: "Editor", label: w.roles.Editor },
  { value: "Owner", label: w.roles.Owner },
];
