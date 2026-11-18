import type { Schemas } from "../../api/client";
import { w } from "../../words";
import type { Option } from "./Select";

export const ROLE_OPTIONS: readonly Option<Schemas["Role"]>[] = [
  { value: "viewer", label: w.roles.viewer },
  { value: "editor", label: w.roles.editor },
  { value: "owner", label: w.roles.owner },
];
