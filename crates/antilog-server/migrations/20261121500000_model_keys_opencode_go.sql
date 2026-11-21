alter table model_keys drop constraint model_keys_provider;
alter table model_keys add constraint model_keys_provider
    check (provider in ('anthropic', 'openai', 'opencode', 'opencode-go'));
