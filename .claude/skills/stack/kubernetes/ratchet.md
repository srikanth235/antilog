# Kubernetes, Helm and OpenTofu in Ratchet

(target) Nothing here is on this branch yet. This page says how Ratchet is packaged and deployed when it lands. Versions are in [libraries.md](../libraries.md).

## What gets built (target)

- **One container image** for `ratchet-server`, with the web app built in, multi-architecture (arm64 and amd64), built with `cargo-chef`, signed with cosign and shipped with an SBOM from syft.
- **One Helm chart**, `deploy/chart/` (target), for every deployment: Ratchet cloud and every client's self-hosted install, on EKS, AKS, GKE, OpenShift or k3s. It holds the server Deployment, a migration Job that runs before an upgrade, the Service and Ingress, the judge sandbox node pool's settings, and a values file per cloud.
- **One OpenTofu module per cloud**, `deploy/tofu/aws/`, `azure/`, `gcp/` (target). It creates what sits around the cluster: managed Postgres (RDS, Azure Database for PostgreSQL, Cloud SQL), a bucket, a key in the cloud's key service, the secret store entries and workload identity for the server's service account. AWS comes first; Azure and GCP are written when the first client on each signs. A client with its own cluster and database uses the chart alone.

## Ratchet cloud, in steps

| Stage | Setup | About, a month |
| --- | --- | --- |
| Demos | One `t4g.medium` EC2 instance running k3s, Postgres in the cluster with a nightly backup to S3, Traefik with Let's Encrypt, stopped between demos by one script | $5 idle |
| First paying client | EKS, RDS, one namespace and one database per client | $200 |
| Clients rely on it | Multi-AZ RDS, nodes across zones, backups and logs | $400 to $500 |

The chart is the same at every stage; only the values and the module change. Platform spend (AgentCore, Bedrock) is per use and on top.

## Rules

- **Nothing in the chart or the server names one cloud.** Cloud differences live in the values file and the OpenTofu module. A cloud-specific annotation goes in that cloud's values file, never in a template.
- **No stored cloud keys.** The server's service account gets cloud access by workload identity (EKS Pod Identity or IRSA, Azure Workload Identity, GKE Workload Identity). Secrets reach the pod as mounted files from the cloud's secret manager through the cluster's secret tooling; the server never calls a secret-manager API.
- **Postgres is outside the chart in production.** The chart can run Postgres in the cluster for demos and local tests only, behind a value that is off by default.
- **Migrations run once, before the new server.** A pre-upgrade Job runs the forward-only migrations; the server refuses to start on a schema newer than it knows. See [../postgres/ratchet.md](../postgres/ratchet.md).
- **The judge sandbox gets its own nodes.** In the Kubernetes sandbox adapter, judge pods run under gVisor or Kata on a dedicated, tainted node pool with fixed instance types, one pair at a time per node, so timing is not shared with other work. Agent pods, if used, get their own pool.
- **Air-gapped installs work.** Every image the chart uses can be mirrored to the client's registry and is referenced by digest.
- **Only CI deploys Ratchet cloud.** No one runs `helm upgrade` or `tofu apply` against a client's deployment from a laptop. A plan is reviewed before an apply. See [../security/ratchet.md](../security/ratchet.md) for the workflow rules.
- **Least privilege in the cluster.** Pods run as non-root with a read-only root filesystem and no added capabilities; a NetworkPolicy allows only the database, object storage, the platforms' endpoints and the identity provider.

## Check

```bash
helm lint deploy/chart                                    # (target)
helm template deploy/chart -f deploy/chart/values-aws.yaml # (target) and each cloud's values
tofu fmt -check -recursive deploy/tofu                    # (target)
tofu -chdir=deploy/tofu/aws validate                      # (target) and each module
```

CI also installs the chart on a k3s cluster and runs a smoke test ([../../land/ci.md](../../land/ci.md)).

## Pitfalls

- **A `latest` tag** makes an upgrade unrepeatable. Reference images by digest.
- **A migration inside the server's start-up** runs once per replica during a rollout. Use the pre-upgrade Job.
- **Liveness probes that check the database** restart every pod when Postgres blips. Liveness checks the process; readiness checks dependencies.
- **Resource requests left unset** let the judge share a node with noisy work and widen the noise band. Set requests and limits, and keep the judge on its own pool.
- **NAT gateways and load balancers cost by the hour** whether or not anyone uses them. Count them before adding one; demos use neither.
