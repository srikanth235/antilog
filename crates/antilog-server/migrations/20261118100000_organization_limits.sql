-- Owner: organization. Spend limits in US cents on the organization row.
set local lock_timeout = '5s';

alter table organizations
    add column monthly_cap_cents bigint check (monthly_cap_cents >= 0),
    add column default_budget_cents bigint check (default_budget_cents >= 0),
    add constraint organizations_default_within_cap
        check (default_budget_cents <= monthly_cap_cents),
    add constraint organizations_id_key unique (id);
