import { useQuery, useQueryClient } from "@tanstack/react-query";
import { Navigate } from "@tanstack/react-router";
import { w } from "../../words";
import { failureOf, finishRedirect } from "./signin";

/** The provider sends the browser here with a code; one call turns it into a session. */
export function Callback(props: { provider: string; code: string; state: string }) {
  const client = useQueryClient();
  const result = useQuery({
    queryKey: ["sign-in", props.provider, props.state],
    queryFn: async () => {
      try {
        await finishRedirect(props.provider, props.code, props.state);
        await client.invalidateQueries();
        return "ok" as const;
      } catch (caught) {
        return failureOf(caught);
      }
    },
    retry: false,
    staleTime: Number.POSITIVE_INFINITY,
  });
  if (result.data === "ok") return <Navigate to="/" replace />;
  if (result.data !== undefined)
    return <Navigate to="/signin" search={{ error: result.data }} replace />;
  return (
    <main className="main" style={{ alignItems: "center", justifyContent: "center" }}>
      <p role="status">{w.signIn.finishing}</p>
    </main>
  );
}
