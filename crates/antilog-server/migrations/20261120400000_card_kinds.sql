-- Owner: session (runner). The runner queue and Customize each rewrote the card checks with their
-- own new kind; this one allows both, and is the last word until the next kind.
set local lock_timeout = '5s';

alter table policies drop constraint policies_card;
alter table policies add constraint policies_card check (card in (
    'seal_experiment', 'raise_budget', 'add_credential', 'attach_repo',
    'change_environment', 'open_pull_request', 'keep_going', 'continue_on_another_agent',
    'ask_first', 'use_connector_tool'));
alter table approvals drop constraint approvals_card;
alter table approvals add constraint approvals_card check (card in (
    'seal_experiment', 'raise_budget', 'add_credential', 'attach_repo',
    'change_environment', 'open_pull_request', 'keep_going', 'continue_on_another_agent',
    'ask_first', 'use_connector_tool'));
