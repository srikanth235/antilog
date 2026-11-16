# Helm charts and Kubernetes manifests (notes)

Source: Ratchet's own notes from Helm's [Chart Best Practices Guide](https://helm.sh/docs/chart_best_practices/) (helm/helm-www, MIT) and the Kubernetes documentation on [Pod Security Standards](https://kubernetes.io/docs/concepts/security/pod-security-standards/), [network policies](https://kubernetes.io/docs/concepts/services-networking/network-policies/), [recommended labels](https://kubernetes.io/docs/concepts/overview/working-with-objects/common-labels/) and [configuration good practices](https://kubernetes.io/docs/concepts/configuration/overview/) (CC BY 4.0, The Kubernetes Authors), read November 2026. No popular agent skill covers Helm well, so this page is written here. [`../ratchet.md`](../ratchet.md) and [infra/rules.md](../../../infra/rules.md) win where they differ.

Ratchet has two charts (target): the server, for customers who self-host on Kubernetes, and the runner, which every Your infrastructure customer installs. Sandbox pods are not in a chart: the runner creates them through the Kubernetes API with the `kube` crate. The pod rules below apply to both, and a test checks the pod specs the runner builds the same way a chart test checks a rendered template.

## Chart layout

- `Chart.yaml` uses `apiVersion: v2`. `version` is the chart's own SemVer and changes with every chart change; `appVersion` is the Ratchet release it installs.
- Chart and template file names are lower case with dashes. One resource kind per template file, named after it (`deployment.yaml`, `networkpolicy.yaml`).
- Named templates live in `_helpers.tpl` and are prefixed with the chart name (`ratchet-runner.labels`), because template names are global across a chart and its dependencies.
- Use `{{-` and `-}}` to keep rendered YAML free of blank lines, and `nindent` for blocks. Quote strings that could be read as numbers or booleans.
- Chart dependencies are pinned to exact versions and the lock file is committed.

## Values

- Names are camelCase, start lower case, and are as flat as reads well. Every value has a comment above it in `values.yaml` saying what it does; that file is the customer's documentation.
- Ship a `values.schema.json`. Helm validates values against it on install, upgrade, lint and template, so a typo fails loudly instead of being ignored.
- Defaults are safe: the most restrictive network policy, no extra permissions, resource limits set. A customer opens things up, never closes them.
- Booleans that turn a block on are named for what they create (`networkPolicy.enabled`, `serviceAccount.create`), with `serviceAccount.name` to use an existing one.
- **No secret values in values.** Helm stores every release, with its rendered manifests and values, in a Secret in the release namespace, so a token in values is copied there. Take the name of an existing Secret (`existingSecret`) that the customer's secret manager fills.

## Labels and images

- Every resource carries the recommended labels: `app.kubernetes.io/name`, `app.kubernetes.io/instance`, `app.kubernetes.io/version`, `app.kubernetes.io/component`, `app.kubernetes.io/part-of: ratchet`, `app.kubernetes.io/managed-by`, and `helm.sh/chart`. Selectors use only `name` and `instance`, which never change on upgrade.
- Sandbox pods also carry the run tags from [`../ratchet.md`](../ratchet.md) as labels. A label key cannot hold `:`, so `ratchet:run` becomes a prefixed key; (target) the prefix is chosen with the first chart. Label values are at most 63 characters of letters, digits, `-`, `_` and `.`; a UUID fits. Clean-up finds pods by these labels.
- Images are referenced by digest in release values, never by `latest` or a moving tag.

## Pods

Every pod meets the Kubernetes **restricted** Pod Security Standard, and the namespaces the runner uses for sandboxes carry `pod-security.kubernetes.io/enforce: restricted` so the cluster refuses anything less:

- `runAsNonRoot: true`, `allowPrivilegeEscalation: false`, `capabilities.drop: [ALL]`, `seccompProfile.type: RuntimeDefault`.
- No `hostPath`, `hostNetwork`, `hostPID` or `hostIPC`. Volumes only of the allowed kinds (`emptyDir`, `configMap`, `secret`, `projected`, `persistentVolumeClaim`, and the like).
- On top of the standard: `readOnlyRootFilesystem: true` with an `emptyDir` for scratch, and `automountServiceAccountToken: false` unless the pod calls the API (the runner does; sandboxes do not).
- Requests and limits for CPU, memory and `ephemeral-storage`. A benchmark pod gets requests equal to limits, so it is scheduled with a guaranteed share and measurements are steadier.
- A sandbox pod has `activeDeadlineSeconds` set to its run's lifetime, so a pod the runner forgets still stops. Jobs get `ttlSecondsAfterFinished`.
- Workspaces and evaluators go to separate node pools with `nodeSelector` and taints and tolerations, as [`../ratchet.md`](../ratchet.md) requires.
- The server has a readiness probe that passes only after migrations have run, and a liveness probe that does not touch the database.

## Network policy

- Each sandbox namespace starts with a policy that denies all ingress and egress, then allows only what the run names: the egress proxy, and the data sources its scoped credential covers. Nothing may reach another sandbox, and a workspace never reaches an evaluator. This is the "no route" in [infra/model.md](../../../infra/model.md).
- DNS is egress too. Either allow the cluster's DNS service explicitly or route names through the egress proxy; a policy that forgets DNS fails in confusing ways.
- **A policy is only as good as the network plugin.** Some network plugins ignore `NetworkPolicy` without an error. (target) The runner checks on install that a deny-all policy actually blocks traffic, and refuses to run sandboxes if it does not.

## Permissions

- The runner's service account gets a namespaced `Role`, not a `ClusterRole`, limited to what it does: create, watch and delete pods, secrets and network policies in its sandbox namespaces. Charts create RBAC only when `rbac.create` is true.
- Sandbox pods have no service account permissions at all.
- Custom resource definitions are not used. Adding one is a design change: ask.

## Helm 4

The pinned version is Helm 4 ([`../../libraries.md`](../../libraries.md)). Advice written for Helm 3 mostly holds; check anything about install and upgrade behaviour, such as how changes are applied and waited for, against the Helm 4 release notes before relying on it.

## Checks

`helm lint --strict` and `helm template` with each values file in the chart's test set, then assertions on the rendered output: every pod passes the rules above, every resource has the labels, the default values render a deny-all network policy. An upgrade from the previous release's values renders without error. Installing into a real cluster is a real-cloud run ([land/real-cloud.md](../../../land/real-cloud.md)). Schema validators and test plugins for Helm are not dependencies unless [`../../libraries.md`](../../libraries.md) lists them.
