# Packer image builds (digest)

Source: the Packer skills (`aws-ami-builder`, `azure-image-builder`, `windows-builder`, `push-to-registry`) in [hashicorp/agent-skills](https://github.com/hashicorp/agent-skills) (plugins/packer/skills), by HashiCorp (MPL-2.0, [`../licenses/hashicorp-agent-skills-LICENSE`](../licenses/hashicorp-agent-skills-LICENSE)). Condensed in our own words. Packer is its own tool, not Terraform, so this applies to Ratchet unchanged; the Terraform digest [terraform.md](terraform.md) applies to OpenTofu. Ratchet's images are Linux, built first for AWS; the Windows and HCP Packer registry skills are not summarised beyond the notes at the end. [`../ratchet.md`](../ratchet.md) wins where they differ.

> A build launches a real machine and so costs money and takes minutes (the source says 10 to 30 minutes for an AMI, longer on other clouds). A failed build can leave the build machine running: always check for leftovers.

## Template shape (HCL2)

- A `packer { required_plugins { amazon = { source = "github.com/hashicorp/amazon", version = "~> 1.3" } } }` block pins each builder plugin. Run `packer init` to install them.
- `variable` blocks for inputs (mark secrets `sensitive`), `locals` for derived values, one `source "amazon-ebs" "<name>" { ... }` per base image and a `build { sources = [...] provisioner ... }` block that says what to do to it.
- The `amazon-ebs` source takes a region, an instance type, a `source_ami_filter` and an `ssh_username`.

## Choosing the base image

- Filter by name pattern, root device type and virtualization, with `most_recent = true` and, always, an `owners` list naming the publisher's account (Canonical's, or `amazon`). Without `owners`, a filter can match an image anyone published.
- Pin what must be reproducible. A "latest" filter makes two builds of the same template differ.

## Naming, tagging, uniqueness

- AMI names must be unique per region. Put a timestamp or a version in the name (`name-${local.timestamp}`, or a `-var image_version=...` input) or the second build fails.
- Tag the image and its snapshots (`Name`, build date, git commit, owner). Tags let a clean-up job find and delete images by age, and make cost visible. `run_tags` and `run_volume_tags` tag the temporary build machine, so a leaked builder can be found.
- `ami_regions` copies the image to other regions after the build.

## Provisioning

- Keep provisioners few and idempotent: update packages, install the one program, set its service up, clean temporary files. Put the long script in a file, not inline.
- Everything a provisioner downloads should be pinned and checked (a version and a checksum). A shell that pipes a download into `sh` makes the image depend on a moving target.
- Do not bake secrets into an image. Credentials arrive at run time, from the platform's identity or secret store.
- Set a sensible timeout for slow steps and the SSH wait.

## Authentication

- Packer uses the cloud SDK's credential chain: environment variables, the shared credentials file, or an instance profile when running on a cloud machine. Prefer a role attached to the CI runner, or short-lived credentials, over long-lived keys in the environment. Give the builder the least privilege that lets it launch, snapshot and tag, in a dedicated account where possible.

## Commands

```bash
packer fmt -check -diff .      # formatting
packer init .                  # install plugins
packer validate .              # syntax and configuration, makes no cloud calls
packer build .                 # builds, and spends money
packer build -var "region=us-east-1" .
```

## Common failures

- **SSH timeout:** the build machine's security group does not allow port 22 from the runner, or its subnet has no route out.
- **Name already exists:** make the image name unique (above).
- **Volume too small:** the base image's volume is larger than the one you asked for; set the root device size in the source.
- **Leaked builder or snapshot after a failed build:** list by the run tags and delete.

## Other skills in the source (not summarised)

- **Windows images** use the WinRM communicator, PowerShell provisioners and long Windows Update runs (the source says 45 to 120 minutes). Not used here.
- **HCP Packer registry** pushes build metadata (not images) to HashiCorp's service for lineage and promotion channels. It is an outside service and a product decision; not adopted.
- **Azure images** use the `azure-arm` builder, with service principal or CLI authentication and an optional compute gallery; relevant when an Azure backend is built.
