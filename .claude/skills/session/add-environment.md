# Add an environment

An environment is a network level, plain variables, secrets and a setup script, with a size (target; see [docs/V0.md](../../../docs/V0.md)). A session runs on one.

1. **A name and an id.** Give the environment a stable id. Experiments refer to it by id, never by a platform's own environment id.
4. **A size.** Name the CPU, memory and disk. An experiment stays on one environment, so measurements compare.
6. **Network.** List the hosts it may reach. A level (none, limited with allowed hosts, full) is set in the environment.
7. **Cost it.** Show the estimated spend per hour on the Change environment card before approval.
8. **Changed by a card.** An agent proposes a change with `set_environment`; a person approves it (see [tools.md](tools.md)). A change that affects what is measured makes a new experiment version, because numbers do not carry across.

## Proving it

Follow [../land/SKILL.md](../land/SKILL.md).
