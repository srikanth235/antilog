class Component extends DCLogic {
  constructor(props) {
    super(props);
    this.setScroll = (el) => { this.scrollEl = el; };
    this.setPalIn = (el) => { this.palIn = el; };
    this.setSdBox = (el) => { this.sdBox = el; };
    this.setMainRef = (el) => { this.sdMain = el; };
    this.K = this.mkK();
    const K = this.K;
    const CP = 'Checkout platform', SR = 'Search', DS = 'Data science forecasts';
    const X = (o) => ({ pinned: false, approved: true, hist: [], pr: [], ps: 0, fixed: null, lim: { error: true, cost: true, memory: false }, dir: null, path: null, env: 'k8s', mt: 'm7i.2xlarge', val: 0, mo: 5, act: '', ...o });
    this.state = {
      screen: 'thread', theme: null, menu: null, collapsed: {}, settingsTab: 'profile', setOpen: false, setQ: '', auditQ: '', roleNote: '', 
      pal: false, palQ: '', palScope: 'all', palHi: 0, palAct: false, palActHi: 0, recent: ['x:o2', 'p:Search', 'd:eval:k6', 'd:skill:pgtune', 's:members', 'x:o6'], hlNote: null,
      selId: 'o1', projSel: CP, lastProject: CP, needText: false, editId: null,
      cTab: 'eval', cView: 'yours', cQuery: '', cCat: 'All', cAuth: 'All', cAdded: 'All', cSort: 'az', cNotice: '', showAll: {}, justAdded: {}, cSlide: 0,
      form: null, ff: { a: '', b: '', c: '' }, fpick: {}, plRemove: null, skSec: 0, reconnect: { sf: true },
      skills: K.SKILLS.filter((k) => k.yours).map((k) => ({ ...k })), skillsOff: {}, plugins: [], pluginRec: {},
      detail: { kind: 'eval', id: 'k6' }, connSel: 'gh', dsSel: 'traces',
      ex: {}, hl: null, confirmStop: false, renaming: false, renameVal: '',
      histTab: 'trials', histProj: 'All', histExp: 'All',
      defCap: '200', newToken: false, noteDraft: '', threadDraft: '', capDraft: '', monthly: 2500,
      d: { text: '', ev: null, dir: null, limits: { error: true, cost: true, memory: false }, path: null, name: null, env: 'k8s', mt: null, agents: 2, cap: '200', project: null, skills: null },
      newSel: { repo: true, traces: true, past: true },
      deploy: 'Ratchet cloud', secretMgr: 'AWS Secrets Manager',
      launcher: { name: 'runner-acme-1', where: 'acme-prod VPC', version: '0.9.2', seen: '12 seconds ago', up: true },
      backends: { k8s: true, aws: true, gcp: false, azure: false },
      evals: K.EVS.map((e) => ({ ...e })),
      agentList: [
        { id: 'claude', name: 'Claude Code', desc: 'Terminal coding agent', provider: 'Anthropic', model: 'claude-sonnet-5-5', via: 'Bedrock endpoint in acme-prod', cat: 'Hosted', used: 18, par: 3, gl: 'g-term' },
        { id: 'codex', name: 'Codex', desc: 'Terminal coding agent', provider: 'OpenAI', model: 'gpt-5-codex', via: 'Key in your secret manager', cat: 'Hosted', used: 11, par: 2, gl: 'g-term' },
        { id: 'grok', name: 'Grok', desc: 'Terminal coding agent', provider: 'xAI', model: 'grok-code-fast', via: 'Acme gateway', cat: 'Hosted', used: 5, par: 2, gl: 'g-spark' }
      ],
      agents: { claude: true, codex: true, grok: true },
      opts: [
        X({ id: 'o1', skills: ['conv', 'nodeprof', 'pgtune'], name: 'Checkout p95', project: CP, req: 'Lower checkout p95 latency without raising error rate', ev: 'k6', runs: 3, step: 5, status: 'Running', cap: 50, agents: ['claude', 'codex', 'grok'], ds: ['repo', 'traces', 'past'], pinned: true, pr: ['n', 'n'], ps: 71, act: '2 min ago', val: 13300,
          hist: [[1, 'claude', 'Cache the tax table', 409], [1, 'codex', 'Batch address validation', 415], [1, 'grok', 'Drop the retry on payment calls', 452], [2, 'claude', 'Compress the order payload', 405], [2, 'codex', 'Move session lookup to memory', 416], [2, 'grok', 'Lazy-load fraud rules', 419]],
          T: [{ a: 'claude', i: 0, v: 344, h: 349, e: '0.4%', c: '+$0.01', m: '+11 MB', k: ['pgtune', 'Connection pooling'] }, { a: 'codex', i: 1, v: 407, e: '0.3%', c: '$0.00' }, { a: 'grok', i: 2, v: 455, e: '1.9%', c: '+$0.02', f: 'e' }] }),
        X({ id: 'o2', skills: ['conv'], name: 'Forecast accuracy', project: CP, req: 'Cut forecast error without slowing the nightly job', ev: 'fcst', runs: 2, step: 8, status: 'Win', cap: 50, agents: ['claude', 'codex', 'grok'], ds: ['repo', 'sales', 'design'], pr: ['n'], ps: 38, fixed: 44, act: 'Yesterday', val: 8200, mo: 5, pinned: false,
          hist: [[1, 'claude', 'Add promotion flags', 18.1], [1, 'codex', 'Tune the learning rate', 18.6]],
          T: [{ a: 'codex', i: 0, v: 16.0, h: 16.2, e: '0.2%', c: '+$0.04' }, { a: 'claude', i: 1, v: 19.5 }, { a: 'grok', i: 2, v: 18.3, e: '0.2%', c: '−$0.01' }] }),
        X({ id: 'o3', skills: ['conv', 'nodeprof'], name: 'CLI cold start', project: CP, req: 'Speed up CLI cold start', ev: 'cli', runs: 1, step: 4, status: 'Running', cap: 50, agents: ['claude', 'codex'], ds: ['repo', 'past'], act: '12 min ago', val: 4400,
          T: [{ a: 'claude', i: 0, v: 145, h: 148, k: ['nodeprof', 'Start-up traces'] }, { a: 'codex', i: 1, v: 181 }] }),
        X({ id: 'o4', skills: ['conv'], name: 'Bundle size', project: CP, req: 'Shrink the production bundle without breaking routes', ev: 'build', runs: 2, step: 8, status: 'No win', cap: 25, agents: ['claude', 'codex'], ds: ['repo', 'past'], pr: ['s'], ps: 24, fixed: 25, act: '3 days ago',
          hist: [[1, 'claude', 'Tree-shake the icon set', 317.4]],
          T: [{ a: 'claude', i: 0, v: 317.1 }, { a: 'codex', i: 1, v: 299.0, h: 322.0 }] }),
        X({ id: 'o5', skills: ['conv'], name: 'Image resize', project: CP, req: 'Reduce CPU spent resizing product images', ev: 'img', runs: 1, step: 8, status: 'Merged', cap: 50, agents: ['grok', 'claude', 'codex'], ds: ['repo', 'corpus', 'past'], fixed: 27, act: '5 days ago', val: 9800, mo: 1,
          T: [{ a: 'grok', i: 0, v: 47.4, h: 47.9, m: '−6 MB', k: ['conv', 'Tests'] }, { a: 'claude', i: 1, v: 60.1 }, { a: 'codex', i: 2, v: 50.2, h: 60.2 }] }),
        X({ id: 'o6', skills: ['conv', 'pgtune'], name: 'Search relevance', project: SR, req: 'Improve search relevance for exact title queries', ev: 'rel', runs: 2, step: 8, status: 'Pending pull request', cap: 50, agents: ['claude', 'codex', 'grok'], ds: ['repo', 'judg', 'qlog'], pr: ['n'], ps: 33, fixed: 38, act: '2 days ago', val: 11200, mo: 4,
          hist: [[1, 'claude', 'Raise the title weight', 0.614], [1, 'codex', 'Add typo tolerance', 0.607]],
          T: [{ a: 'claude', i: 0, v: 0.665, h: 0.661 }, { a: 'codex', i: 1, v: 0.628 }, { a: 'grok', i: 2, v: 0.598, e: '0.9%', c: '+$0.01', f: 'e' }] }),
        X({ id: 'o7', skills: ['conv'], name: 'Autocomplete latency', env: 'aws', mt: 'c7i.2xlarge', project: SR, req: 'Lower autocomplete p95 latency', ev: 'qry', runs: 1, step: 6, status: 'Running', cap: 25, agents: ['claude', 'codex'], ds: ['repo', 'qlog'], act: '40 min ago',
          T: [{ a: 'codex', i: 0, v: 79, h: 80, e: '0.1%', c: '+$0.01' }, { a: 'claude', i: 1, v: 94 }] }),
        X({ id: 'o8', skills: ['conv'], name: 'Feature pipeline cost', env: 'aws', mt: 'c7i.2xlarge', project: DS, req: 'Cut the cost of the nightly feature pipeline', ev: 'pipe', runs: 1, step: 8, status: 'Merged', cap: 25, agents: ['grok', 'codex'], ds: ['feat', 'sales', 'past'], fixed: 15, act: 'Last week', val: 6100, mo: 3,
          T: [{ a: 'grok', i: 0, v: 12.7, h: 12.9, e: '0.0%', c: '−$5.70' }, { a: 'codex', i: 1, v: 17.95, e: '0.0%', c: '−$0.45' }] })
        ,
        X({ id: 'o9', skills: ['conv', 'flaky'], name: 'Smoke suite duration', project: CP, req: 'Cut the staging smoke suite duration without dropping coverage', ev: 'smoke', runs: 1, step: 0, status: 'Draft', approved: false, cap: 25, agents: ['claude', 'codex'], ds: ['repo', 'past'], act: '20 min ago', T: [] })
      ],
      datasets: K.DATASETS.map((x) => ({ ...x })),
      grants: {},
      connectors: [
        { id: 'gh', name: 'GitHub acme/checkout', desc: 'Repository, design notes and CLI fixtures agents read', cat: 'Code', kind: 'GitHub', used: 14, added: false, gl: 'g-git' },
        { id: 'sf', name: 'Snowflake ANALYTICS', desc: 'Warehouse tables and the holdout datasets', cat: 'Data', kind: 'Snowflake', used: 9, added: false, gl: 'g-db' },
        { id: 'dd', name: 'Datadog monitoring', desc: 'Production traces, query logs and service metrics', cat: 'Monitoring', kind: 'Datadog', used: 11, added: false, gl: 'g-chart' },
        { id: 'rh', name: 'Ratchet run history', desc: 'Every past trial, built in', cat: 'Data', kind: 'Ratchet', used: 31, added: false, gl: 'g-file' }
      ],
      notes: [
        { id: 'n1', text: 'The payment gateway is the slow part. Prefer fewer round trips.', author: 'Marco Bianchi', opt: 'Checkout p95', time: '02:05', run: 3 },
        { id: 'n2', text: 'Holiday calendar features helped last time, try them again.', author: 'Jun Park', opt: 'Forecast accuracy', time: 'Yesterday', run: 2 }
      ],
      machines: [
        { id: 'm1', name: 'ws-checkout-3a', role: 'workspace', opt: 'Checkout p95', be: 'Kubernetes', age: '52 m', gone: false },
        { id: 'm2', name: 'eval-checkout-3', role: 'evaluator', opt: 'Checkout p95', be: 'Kubernetes', age: '14 m', gone: false },
        { id: 'm3', name: 'ws-cli-1', role: 'workspace', opt: 'CLI cold start', be: 'Kubernetes', age: '9 m', gone: false },
        { id: 'm4', name: 'ws-autocomplete-1', role: 'workspace', opt: 'Autocomplete latency', be: 'AWS VMs', age: '58 m', gone: false },
        { id: 'm5', name: 'eval-autocomplete-1', role: 'evaluator', opt: 'Autocomplete latency', be: 'AWS VMs', age: '31 m', gone: false },
        { id: 'm6', name: 'ws-forecast-2', role: 'workspace', opt: 'Forecast accuracy', be: 'Kubernetes', age: '1 h 12 m', gone: true },
        { id: 'm7', name: 'eval-forecast-2', role: 'evaluator', opt: 'Forecast accuracy', be: 'Kubernetes', age: '44 m', gone: true },
        { id: 'm8', name: 'ws-bundle-2', role: 'workspace', opt: 'Bundle size', be: 'Kubernetes', age: '58 m', gone: true },
        { id: 'm9', name: 'eval-bundle-2', role: 'evaluator', opt: 'Bundle size', be: 'Kubernetes', age: '33 m', gone: true },
        { id: 'm10', name: 'ws-image-1', role: 'workspace', opt: 'Image resize', be: 'Kubernetes', age: '1 h 04 m', gone: true },
        { id: 'm11', name: 'eval-image-1', role: 'evaluator', opt: 'Image resize', be: 'Kubernetes', age: '39 m', gone: true },
        { id: 'm12', name: 'ws-search-2', role: 'workspace', opt: 'Search relevance', be: 'Kubernetes', age: '1 h 20 m', gone: true },
        { id: 'm13', name: 'eval-search-2', role: 'evaluator', opt: 'Search relevance', be: 'Kubernetes', age: '47 m', gone: true },
        { id: 'm14', name: 'ws-pipeline-1', role: 'workspace', opt: 'Feature pipeline cost', be: 'AWS VMs', age: '52 m', gone: true },
        { id: 'm15', name: 'eval-pipeline-1', role: 'evaluator', opt: 'Feature pipeline cost', be: 'AWS VMs', age: '28 m', gone: true }
      ],
      roles: { priya: 'Owner', marco: 'Editor', jun: 'Editor', ci: 'Viewer' },
      session: '12 hours',
      tokens: [
        { id: 't1', name: 'Laptop CLI', tail: '3e1b', kind: 'Personal', scope: 'Everything you can', used: '2 hours ago', expires: 'in 81 days' },
        { id: 't2', name: 'ci-bot', tail: '7f2a', kind: 'Service principal', scope: 'Start and read runs', used: 'Monday', expires: 'in 26 days' },
        { id: 't3', name: 'Claude MCP', tail: 'c09d', kind: 'Personal', scope: 'Read only', used: 'Yesterday', expires: 'in 12 days' }
      ],
      audit: [
        { time: '02:05', who: 'Marco Bianchi', action: 'note.send', resource: 'Checkout p95 · note' },
        { time: '00:49', who: 'launcher', action: 'credential.mint', resource: 'Checkout p95 · run #3 · agents read-only, evaluator with holdout' },
        { time: '00:48', who: 'scheduler', action: 'run.start', resource: 'Checkout p95 · run #3' },
        { time: 'Yesterday', who: 'Priya Raman', action: 'pull_request.open', resource: 'Search relevance · PR #298' },
        { time: 'Yesterday', who: 'Jun Park', action: 'grant.change', resource: 'Past runs · Evaluator access removed' },
        { time: 'Mon', who: 'Priya Raman', action: 'budget.change', resource: 'Checkout p95 · $50 per run' },
        { time: 'Mon', who: 'ci-bot (service principal)', action: 'token.revoke', resource: 'old-ci · ends 19d4' },
        { time: 'Last week', who: 'Priya Raman', action: 'launcher.connect', resource: 'runner-acme-1 · acme-prod VPC' }
      ]
    };
    this.state.grants = this.defaultGrants(this.state.datasets);
  }
  defaultGrants(list) {
    const g = {};
    list.forEach((x) => { g[x.id] = x.marking === 'Holdout' ? { agents: false, evaluator: true } : { agents: true, evaluator: true }; });
    g.past = { agents: true, evaluator: false };
    g.design = { agents: false, evaluator: false };
    return g;
  }
  componentDidMount() {
    this._key = '';
    this._loc = this.locKey(this.state);
    this._onKey = (e) => this.onKey(e);
    window.addEventListener('keydown', this._onKey);
    this.afterRender();
  }
  componentWillUnmount() { window.removeEventListener('keydown', this._onKey); }
  componentDidUpdate() {
    const k = this.locKey(this.state);
    if (k !== this._loc) {
      const prev = this._loc;
      this._loc = k;
      if (prev) this.setState((st) => ({ recent: [prev, ...st.recent.filter((q) => q !== prev && q !== k)].slice(0, 10) }));
    }
    this.afterRender();
  }
  locKey(s) {
    return s.screen === 'thread' ? 'x:' + s.selId : s.screen === 'project' ? 'p:' + s.projSel : s.screen === 'detail' ? 'd:' + s.detail.kind + ':' + s.detail.id : ['projects', 'value', 'history'].includes(s.screen) ? 'g:' + s.screen : null;
  }
  afterRender() {
    const s = this.state;
    if (s.pal && !this._palWas) {
      this._palWas = true;
      if (this.palIn) this.palIn.focus();
    } else if (!s.pal && this._palWas) {
      this._palWas = false;
      const pf = this._prevFocus;
      if (!this._compose && pf && pf.focus && document.contains(pf)) pf.focus();
      this._compose = false;
    }
    if (s.pal && this._hiKb) {
      this._hiKb = false;
      const r = document.getElementById('pr-' + (this._pal ? this._pal.hi : 0));
      if (r && r.scrollIntoView) r.scrollIntoView({ block: 'nearest' });
    }
    if (s.setOpen && !this._setWas) {
      this._setWas = true;
      this._setKey = '';
      if (this.sdBox) this.sdBox.focus();
    } else if (!s.setOpen && this._setWas) {
      this._setWas = false;
      let ft = this._setPrev;
      if (!ft || !ft.focus || !document.contains(ft)) ft = document.querySelector(this._setFall || '#btn-user');
      if (ft && ft.focus) ft.focus();
      this._setPrev = null;
    }
    if (s.setOpen && this.sdMain && this._effTab !== this._setKey) { this._setKey = this._effTab; this.sdMain.scrollTop = 0; }
    if (this._scrollTo && /^sr-/.test(this._scrollTo)) {
      const rg = document.getElementById(this._scrollTo);
      if (rg) { this._scrollTo = null; if (rg.scrollIntoView) rg.scrollIntoView({ block: 'center' }); rg.classList.add('flash'); setTimeout(() => rg.classList.remove('flash'), 2500); }
    }
    const el = this.scrollEl;
    if (!el) return;
    const o = s.opts.find((x) => x.id === s.selId);
    const key = s.screen === 'thread' && o
      ? ['t', o.id, o.step, o.status, o.runs, o.T.length, s.notes.filter((n) => n.opt === o.name).length, o.lim.memory ? 1 : 0, o.approved ? 1 : 0].join(':')
      : 's:' + s.screen + ':' + ':' + (s.detail ? s.detail.id : '') + s.projSel;
    if (key !== this._key) {
      this._key = key;
      el.scrollTop = s.screen === 'thread' ? el.scrollHeight : 0;
    }
    if (this._scrollTo) {
      const tg = document.getElementById(this._scrollTo);
      if (tg) { this._scrollTo = null; if (tg.scrollIntoView) tg.scrollIntoView({ block: 'center' }); }
    }
  }
  mkK() {
    const GEN = [
      ['Batch the slowest calls and reuse connections', 3, 36, 14, ['@@ src/client.ts', '-  for (const id of ids) await fetchOne(id);', '+  await fetchMany(ids);', '+  const agent = new Agent({ keepAlive: true });']],
      ['Cache repeated lookups within a request', 2, 24, 5, ['@@ src/lookup.ts', '+const seen = new Map();', ' export async function get(k) {', '+  if (seen.has(k)) return seen.get(k);', '   return load(k);']],
      ['Remove an unneeded retry and a redundant check', 2, 9, 22, ['@@ src/call.ts', '-  return retry(() => send(req), 3);', '+  return send(req);', '-  validate(req);']]
    ];
    const EVS = [
      { id: 'k6', name: 'k6 checkout load', cat: 'Speed', desc: 'Replays checkout traffic against a fresh machine and reports request latency percentiles', metric: 'Checkout p95 latency', fmt: ['', ' ms', 0], dir: 'lower', base: 412, n: 2.1, cmd: 'k6 run bench/checkout.js --vus 200 --duration 5m', cal: 'Today', runs: 9, used: 14, path: 'services/checkout', ho: 'ho_checkout', kw: 'checkout latency p95 p99 request api payment cart',
        ideas: [['Pool size 20 to 48, reuse clients', 3, 41, 17, ['@@ src/db/pool.ts', '-  max: 20,', '+  max: 48,', '+  idleTimeoutMillis: 30000,', '@@ src/payments/client.ts', '-  const c = new GatewayClient(cfg);', '+  const c = clients.get(cfg.id) ?? makeClient(cfg);']],
          ['Cache pricing lookups per request', 2, 28, 6, ['@@ src/pricing/lookup.ts', '+const memo = new Map<string, Price>();', ' export async function price(sku: string) {', '-  return fetchPrice(sku);', '+  if (!memo.has(sku)) memo.set(sku, await fetchPrice(sku));', '+  return memo.get(sku)!;']],
          ['Inline tax calculation, drop retry', 4, 52, 31, ['@@ src/tax/calc.ts', '-  const t = await taxService.quote(order);', '+  const t = order.total * 0.0825;', '@@ src/payments/charge.ts', '-  return retry(() => gateway.charge(o), 3);', '+  return gateway.charge(o);']]] },
      { id: 'fcst', name: 'Forecast backtest', cat: 'Quality', desc: 'Forecast error against held-out periods', metric: 'Forecast error (MAPE)', fmt: ['', '%', 1], dir: 'lower', base: 18.4, n: 2.6, cmd: 'python -m forecast.backtest --folds 6', cal: 'Last week', runs: 9, used: 3, path: 'models/forecast', ho: 'ho_fcst', kw: 'forecast error mape accuracy demand model',
        ideas: [['Seasonal features from the holiday calendar, boosting depth 6 to 8', 2, 34, 4, ['@@ models/forecast/features.py', '+from calendars import holidays', '+df["is_holiday"] = df.date.isin(holidays("US"))', '+df["days_to_holiday"] = days_to_next(df.date)', '@@ models/forecast/train.py', '-    max_depth=6,', '+    max_depth=8,']],
          ['Swap to a linear model', 1, 6, 6, ['@@ models/forecast/train.py', '-model = GradientBoostingRegressor(max_depth=6)', '+model = Ridge(alpha=1.0)']],
          ['Drop sparse SKUs from training', 1, 3, 0, ['@@ models/forecast/data.py', '+    df = df.groupby("sku").filter(lambda g: len(g) >= 30)']]] },
      { id: 'cli', name: 'CLI cold start', cat: 'Speed', desc: 'Time from launch to first output for a command line tool', metric: 'Startup time, p50', fmt: ['', ' ms', 0], dir: 'lower', base: 184, n: 6.4, cmd: 'hyperfine --warmup 3 "acme --version"', cal: 'Yesterday', runs: 9, used: 8, path: 'cli/src', ho: 'ho_cli', kw: 'cli cold start startup launch boot',
        ideas: [['Lazy-load the plugin registry', 2, 18, 9, ['@@ cli/src/main.ts', '-import { registry } from "./plugins/registry";', '+const registry = () => import("./plugins/registry");', '@@ cli/src/run.ts', '-  registry.load(cmd);', '+  (await registry()).load(cmd);']],
          ['Precompile the config schema', 2, 22, 3, ['@@ cli/src/config.ts', '-const validate = compile(schema);', '+import validate from "./config.validate.js";']]] },
      { id: 'build', name: 'Build size', cat: 'Size', desc: 'Gzip size of the production bundle after a clean build', metric: 'Gzip bundle size', fmt: ['', ' KB', 1], dir: 'lower', base: 318.0, n: 0.4, cmd: 'pnpm build && node scripts/size.js', cal: 'Today', runs: 9, used: 9, path: 'web/src', ho: 'ho_build', kw: 'bundle size build gzip weight bytes',
        ideas: [['Replace the date library', 3, 14, 31, ['@@ web/src/format.ts', '-import { format } from "date-fns";', '+const fmt = new Intl.DateTimeFormat("en-US");', '-  return format(d, "PP");', '+  return fmt.format(d);']],
          ['Split the checkout route', 2, 11, 3, ['@@ web/src/routes.tsx', '-import Checkout from "./pages/Checkout";', '+const Checkout = lazy(() => import("./pages/Checkout"));', '@@ web/vite.config.ts', '+  manualChunks: { checkout: ["./src/pages/Checkout"] },']]] },
      { id: 'img', name: 'Image corpus resize', cat: 'Speed', desc: 'CPU time per resize across a fixed corpus of product images', metric: 'CPU per resize', fmt: ['', ' ms', 1], dir: 'lower', base: 61.0, n: 1.4, cmd: 'cargo bench --bench resize -- --corpus corpus/v6', cal: 'Today', runs: 9, used: 4, path: 'img/src', ho: 'ho_img', kw: 'image resize cpu thumbnail jpeg',
        ideas: [['Resize from the embedded JPEG preview', 2, 19, 8, ['@@ img/resize.rs', '-    let img = decode_full(&bytes)?;', '+    let img = match exif_preview(&bytes) {', '+        Some(p) if p.width >= target.w => decode(p)?,', '+        _ => decode_full(&bytes)?,', '+    };']],
          ['Reuse decode buffers', 1, 7, 2, ['+thread_local! { static BUF: RefCell<Vec<u8>> = RefCell::new(Vec::new()); }', '-    let mut out = Vec::with_capacity(len);', '+    let mut out = BUF.with(|b| b.take());']],
          ['Use a cheaper filter for large reductions', 1, 3, 3, ['@@ img/resize.rs', '-    let f = FilterType::Lanczos3;', '+    let f = FilterType::Triangle;']]] },
      { id: 'rel', name: 'Relevance judgments', cat: 'Quality', desc: 'NDCG against a labelled set of queries and results', metric: 'Search relevance (NDCG)', fmt: ['', '', 3], dir: 'higher', base: 0.610, n: 1.6, cmd: 'python -m search.eval --judgments v4', cal: 'Yesterday', runs: 9, used: 6, path: 'search/rank', ho: 'ho_rel', kw: 'search relevance ndcg ranking results',
        ideas: [['Boost exact title matches, drop stale click signals', 2, 12, 3, ['@@ search/rank.py', '+    score += 2.0 * exact_title_match(q, doc)', '-    score += 0.4 * clicks_90d(doc)', '+    score += 0.4 * clicks_30d(doc)']],
          ['Add a synonym expansion step', 2, 31, 0, ['@@ search/query.py', '+    q = expand_synonyms(q, SYNONYMS)']],
          ['Rerank with recency', 1, 5, 0, ['@@ search/rank.py', '+    score *= 1 + 0.2 * recency(doc.updated_at)']]] },
      { id: 'qry', name: 'Query replay', cat: 'Speed', desc: 'Request latency percentiles under a replayed query load', metric: 'Autocomplete p95 latency', fmt: ['', ' ms', 0], dir: 'lower', base: 96, n: 3.0, cmd: 'k6 run bench/suggest.js --vus 100', cal: 'Today', runs: 9, used: 12, path: 'services/search', ho: 'ho_qry', kw: 'autocomplete latency suggest query search p95',
        ideas: [['Prefix cache in front of the index', 2, 26, 4, ['@@ search/suggest.go', '+var cache = lru.New(50_000)', ' func Suggest(p string) []Hit {', '+    if h, ok := cache.Get(p); ok { return h }', '     hits := index.Prefix(p, 8)', '+    cache.Add(p, hits)']],
          ['Shrink the result page size', 1, 2, 2, ['@@ search/suggest.go', '-    hits := index.Prefix(p, 20)', '+    hits := index.Prefix(p, 8)']]] },
      { id: 'pipe', name: 'Pipeline cost report', cat: 'Cost', desc: 'Compute and warehouse cost of one nightly pipeline run', metric: 'Pipeline cost per run', fmt: ['$', '', 2], dir: 'lower', base: 18.4, n: 2.2, cmd: 'python -m pipelines.cost --run nightly', cal: 'Last week', runs: 9, used: 3, path: 'pipelines/features', ho: 'ho_pipe', kw: 'pipeline cost spend warehouse features',
        ideas: [['Cache joined tables between feature steps', 2, 17, 2, ['@@ pipelines/features/join.py', '+@cache_table(ttl="6h")', ' def join_orders_users(ctx):']],
          ['Partition by day, skip unchanged days', 2, 21, 6, ['@@ pipelines/features/run.py', '-    build(all_days)', '+    build([d for d in all_days if changed(d)])']]] },
      { id: 'smoke', name: 'Staging smoke test', cat: 'Quality', desc: 'Pass rate and duration of the staging smoke suite', metric: 'Smoke suite duration', fmt: ['', ' s', 0], dir: 'lower', base: 240, n: 1.2, cmd: 'pnpm test:smoke --env staging', cal: 'Yesterday', runs: 7, used: 3, path: 'tests/smoke', ho: 'ho_checkout', kw: 'smoke test staging suite pass rate duration' }
    ].map((e) => ({ ...e, ideas: e.ideas || GEN }));
    const H = (id, name, conn, v) => ({ id, name, conn, version: v, marking: 'Holdout', sz: '' });
    const DATASETS = [
      { id: 'repo', name: 'Repository', conn: 'gh', version: 'main@9f3c1e7', marking: 'None', sz: '1,204 files', vers: [['main@9f3c1e7', 'Today', '1,204 files'], ['main@41ad90b', '3 days ago', '1,198 files'], ['main@c07d1f2', 'Last week', '1,187 files']] },
      { id: 'design', name: 'Design notes', conn: 'gh', version: 'v3', marking: 'None', sz: '42 pages' },
      { id: 'traces', name: 'Production traces v12', conn: 'dd', version: 'v12', marking: 'None', sz: '4.1M spans' },
      { id: 'qlog', name: 'Query logs', conn: 'dd', version: 'v5', marking: 'None', sz: '9.8M queries' },
      { id: 'past', name: 'Past runs', conn: 'rh', version: 'v31', marking: 'None', sz: '186 attempts' },
      { id: 'sales', name: 'Sales history', conn: 'sf', version: 'v7', marking: 'None', sz: '212M rows' },
      { id: 'feat', name: 'Feature tables', conn: 'sf', version: 'v9', marking: 'None', sz: '38 tables' },
      { id: 'judg', name: 'Relevance judgments', conn: 'sf', version: 'v4', marking: 'None', sz: '14k queries' },
      { id: 'corpus', name: 'Image corpus', conn: 'sf', version: 'v6', marking: 'None', sz: '60k images' },
      { ...H('ho_checkout', 'Checkout replay holdout v4', 'sf', 'v4'), sz: '820k requests' },
      { ...H('ho_fcst', 'Forecast holdout v2', 'sf', 'v2'), sz: '6 periods' },
      { ...H('ho_cli', 'CLI startup holdout v1', 'gh', 'v1'), sz: '40 fixtures' },
      { ...H('ho_build', 'Bundle holdout v1', 'gh', 'v1'), sz: '12 routes' },
      { ...H('ho_img', 'Image corpus holdout v3', 'sf', 'v3'), sz: '9k images' },
      { ...H('ho_rel', 'Relevance holdout v2', 'sf', 'v2'), sz: '3k queries' },
      { ...H('ho_qry', 'Query replay holdout v1', 'dd', 'v1'), sz: '1.2M queries' },
      { ...H('ho_pipe', 'Pipeline holdout v1', 'sf', 'v1'), sz: '14 days' }
    ];
    const BY = (b) => (b === 'R' ? 'by Ratchet' : b === 'T' ? 'Your team' : 'Community');
    const UN = (u) => ' · used in ' + u + (u === 1 ? ' experiment' : ' experiments');
    const G = (k, n, d, c, by, u, x) => ({ id: k, name: n, desc: d, cat: c, byk: by, by: BY(by) + UN(u), used: u, ...x });
    const SK = (id, name, desc, cat, byk, used, ver, ev, sec, x) => ({ id, name, desc, cat, byk, used, ver, ev, sec, gl: 'g-book', by: BY(byk) + UN(used), ...x });
    const PL = (id, name, desc, cat, byk, used, items) => ({ id, name, desc, cat, byk, used, gl: 'g-blocks', by: BY(byk) + UN(used), items: { eval: [], skill: [], conn: [], agent: [], ...items } });
    const SKILLS = [
      SK('conv', 'Team conventions (from repository)', 'Naming, error handling and test layout rules read from your repository', 'Conventions', 'T', 0, '1.3', ['*'], [['CONVENTIONS.md', 'Overview', 'Read from docs/CONTRIBUTING.md and .editorconfig at main@9f3c1e7. Agents follow these rules in every change they propose.'], ['CONVENTIONS.md', 'Tests', 'Tests live next to the code they cover. A change that touches a module updates or adds a test in the same folder.'], ['CONVENTIONS.md', 'Errors', 'Return typed errors from services. Never swallow an exception to make a measurement pass.']], { yours: true }),
      SK('nodeprof', 'Node.js profiling', 'Read a CPU profile, find hot async paths and avoid changes that only move time elsewhere', 'Speed', 'R', 14, '2.1', ['k6', 'cli', 'qry', 'http'], [['SKILL.md', 'Overview', 'Profile before editing. Start from the flame chart, pick the widest frame that your editable paths own, and change only that.'], ['SKILL.md', 'Start-up traces', 'For start-up time, run with --cpu-prof and sort by self time. Defer any module that is not needed before the first output.'], ['examples/pool.md', 'Connection pooling', 'A pool of 20 starved under 200 virtual users. Raising it to 48 and reusing clients removed the wait at the gateway.']], { yours: true }),
      SK('pgtune', 'Postgres query tuning', 'Plans, indexes and round trips: how to speed up a slow query without changing its result', 'Data', 'R', 11, '1.8', ['k6', 'sql', 'qry', 'http'], [['SKILL.md', 'Overview', 'Read EXPLAIN ANALYZE first. Count round trips before counting milliseconds; batching often beats tuning a single query.'], ['SKILL.md', 'Indexes', 'Add an index only when the plan shows a sequential scan on a large table. Check write cost against the guardrail metrics.'], ['examples/batching.md', 'Batching lookups', 'Replace a loop of single-row selects with one query using ANY($1). Keep the result order stable.']], { yours: true }),
      SK('flaky', 'Flaky test triage', 'Find, quarantine and fix tests that fail then pass on retry', 'Quality', 'R', 6, '1.2', ['flaky', 'tests', 'smoke'], [['SKILL.md', 'Overview', 'A flaky test is a timing or shared-state bug until proven otherwise. Reproduce with a fixed seed and 20 repeats.'], ['SKILL.md', 'Fixes that hold', 'Replace sleeps with explicit waits, isolate shared fixtures, and never raise a retry count to hide a failure.']], { yours: true }),
      SK('cache', 'Cache design', 'Where to cache, how long, and how to invalidate without serving stale prices', 'Speed', 'R', 9, '1.5', ['k6', 'qry', 'web', 'http'], [['SKILL.md', 'Overview', 'Cache the read that repeats within one request first. Widen the scope only when hit rate justifies the staleness.'], ['SKILL.md', 'Invalidation', 'Key by every input that changes the answer. Prefer short lifetimes over clever invalidation.']]),
      SK('bundle', 'Bundle splitting playbook', 'Route-level splitting, shared chunks and dependency swaps that cut gzip size', 'Size', 'R', 8, '1.4', ['build', 'web'], [['SKILL.md', 'Overview', 'Split by route before splitting by library. Check that shared code lands in one chunk, not many.'], ['SKILL.md', 'Dependency swaps', 'Replace a large library only when its API surface in your code is small. Confirm routes still render.']]),
      SK('rustalloc', 'Rust allocation hygiene', 'Reuse buffers, avoid clones in hot loops and keep allocations off the critical path', 'Speed', 'C', 4, '0.9', ['img'], [['SKILL.md', 'Overview', 'Measure allocations with a counting allocator. Reuse buffers across iterations with thread-local storage.'], ['SKILL.md', 'Clones', 'Borrow instead of cloning in hot loops. Return slices, not Vec, when callers only read.']]),
      SK('fcfeat', 'Forecast feature engineering', 'Calendar, lag and promotion features that tend to help demand forecasts', 'Quality', 'C', 5, '1.1', ['fcst'], [['SKILL.md', 'Overview', 'Start with calendar features, then lags, then promotions. Validate each on the backtest folds, never on the holdout.'], ['SKILL.md', 'Leakage', 'A feature that uses information from after the forecast date is leakage. Drop it even if the error falls.']]),
      SK('prompt', 'Prompt and answer hygiene', 'Write prompts that are easy to score and keep answer formats stable', 'Quality', 'C', 3, '0.8', ['llm'], [['SKILL.md', 'Overview', 'Keep one instruction per paragraph and ask for a fixed answer shape so the evaluator can score it.'], ['SKILL.md', 'Regressions', 'Re-run the labelled set after every prompt edit. A gain on one slice that loses another is not a gain.']]),
      SK('ci', 'CI caching tactics', 'Dependency, build and test caches that cut wall time without stale results', 'Speed', 'R', 5, '1.2', ['bt', 'tests'], [['SKILL.md', 'Overview', 'Key caches by lockfile hash. Cache the slowest step first and measure on a cold machine.'], ['SKILL.md', 'Pitfalls', 'A cache that hides a broken build is a regression. Always run one uncached build before accepting.']]),
      SK('cost', 'Cloud cost review checklist', 'Where spend hides: idle capacity, wide scans and repeated recomputation', 'Cost', 'R', 4, '1.0', ['cost', 'pipe'], [['SKILL.md', 'Overview', 'Rank services by daily cost, then look for recomputation that a cache or a partition would remove.'], ['SKILL.md', 'Warehouses', 'Prune by partition, select only the columns you need, and avoid cross joins in nightly jobs.']]),
      SK('rank', 'Ranking signal tuning', 'Weights, freshness and exact-match boosts for search relevance', 'Quality', 'C', 3, '0.7', ['rel'], [['SKILL.md', 'Overview', 'Change one weight at a time and read per-query deltas, not only the average NDCG.'], ['SKILL.md', 'Freshness', 'Prefer recency as a gentle multiplier. A hard cutoff hurts long-tail queries.']]),
      SK('cold', 'Cold start checklist', 'Defer work, trim imports and precompute schemas to shorten start-up', 'Speed', 'R', 5, '1.3', ['cli', 'mstart'], [['SKILL.md', 'Overview', 'List everything that runs before the first output. Move the rest behind first use.'], ['SKILL.md', 'Schemas', 'Precompile validators at build time instead of parsing schemas on launch.']])
    ];
    const PLUGINS = [
      PL('webperf', 'Web performance', 'Measure and speed up page load, HTTP latency and build size with checks front-end teams reuse', 'Performance', 'R', 12, { eval: ['http', 'web', 'build'], skill: ['bundle', 'cache'], conn: ['gf'] }),
      PL('backend', 'Backend latency', 'Replay real traffic, find the slow call and tune queries, pools and caches', 'Performance', 'R', 15, { eval: ['k6', 'qry', 'rpc'], skill: ['pgtune', 'nodeprof', 'cache'], conn: ['pg', 'prom'], agent: ['claude'] }),
      PL('forecast', 'Forecasting', 'Backtest forecast error on held-out periods with feature know-how for demand models', 'Quality', 'R', 6, { eval: ['fcst'], skill: ['fcfeat'], conn: ['bq'] }),
      PL('llmq', 'LLM quality', 'Score model answers against a labelled set and keep prompts easy to evaluate', 'Quality', 'C', 4, { eval: ['llm'], skill: ['prompt'], agent: ['claude', 'gemini'] }),
      PL('buildci', 'Build and CI speed', 'Cut clean build time and test suite duration, and quiet flaky tests', 'Performance', 'R', 9, { eval: ['bt', 'tests', 'flaky'], skill: ['ci', 'flaky'], conn: ['gh'] }),
      PL('cloudcost', 'Cloud cost', 'Read billing exports, rank spend and find recomputation worth removing', 'Cost', 'R', 5, { eval: ['cost', 'pipe'], skill: ['cost'], conn: ['bq', 'obj'] }),
      PL('searchrel', 'Search relevance', 'Judge ranking quality and autocomplete speed against labelled queries', 'Quality', 'C', 4, { eval: ['rel', 'qry'], skill: ['rank'], conn: ['pg'] }),
      PL('mobile', 'Mobile start-up', 'Time cold starts on a reference device and trim what runs before first paint', 'Performance', 'C', 3, { eval: ['mstart'], skill: ['cold'], conn: ['sen'] }),
      PL('pipes', 'Data pipelines', 'Track the cost and duration of nightly pipelines and the tables they build', 'Cost', 'R', 4, { eval: ['pipe', 'sql'], skill: ['cost', 'pgtune'], conn: ['sf', 'bq'] }),
      PL('media', 'Image and media', 'Measure resize and encode cost across a fixed corpus of images', 'Performance', 'C', 2, { eval: ['img'], skill: ['rustalloc'], conn: ['obj'] })
    ];
    return {
      EVS, DATASETS, GEN,
      MIN: [0, 1, 6, 9, 38, 52, 60, 66, 72],
      FR: [0, 0.02, 0.08, 0.12, 0.56, 0.76, 0.8, 0.82, 0.84],
      STEPS: [['opt', 'Experiment', 'Start run'], ['inf', 'Compute', 'Create workspace'], ['ctx', 'Data', 'Mint scoped credentials'], ['inf', 'Compute', 'Agents write and run code'], ['inf', 'Compute', 'Evaluator measures, holdout included'], ['opt', 'Experiment', 'Judge against acceptance criteria'], ['ctx', 'Data', 'Store history, open pull request'], ['inf', 'Compute', 'Clean up']],
      COL: { opt: 'var(--accent)', ctx: 'var(--good)', inf: 'var(--infra)' },
      STAT: { 'Draft': ['Draft', 'st-draft', 'var(--ink3)', 'st'], 'Running': ['Running', 'st-run', 'var(--accent)', 'st run'], 'Win': ['Win', 'st-win', 'var(--good)', 'st'], 'Pending pull request': ['Pull request open', 'st-pr', 'var(--accent)', 'st'], 'Merged': ['Merged', 'st-merged', 'var(--good)', 'st'], 'No win': ['No win', 'st-none', 'var(--ink3)', 'st'], 'Stopped': ['Stopped', 'st-stop', 'var(--ink3)', 'st'] },
      OUT: { accepted: ['Accepted', 'var(--good)'], pend: ['Beats noise', 'var(--good)'], noise: ['Within noise', 'var(--ink3)'], worse: ['Worse', 'var(--bad)'], holdout: ['Failed holdout', 'var(--warn)'], measuring: ['Measuring', 'var(--ink3)'], disc: ['Discarded', 'var(--ink3)'] },
      PROJECTS: [['Checkout platform', 'Owner', 'Latency, forecasting and build health of the checkout service'], ['Search', 'Editor', 'Relevance and speed of search and autocomplete'], ['Data science forecasts', 'Viewer', 'Feature pipelines and cost of the forecasting models']],
      PEOPLE: { 'Priya Raman': 'P', 'Marco Bianchi': 'M', 'Jun Park': 'J' },
      AGN: { claude: 'Claude Code', codex: 'Codex', grok: 'Grok', gemini: 'Gemini CLI', aider: 'Aider', openhands: 'OpenHands' },
      LIMITS: [['error', 'Error rate'], ['cost', 'Cost'], ['memory', 'Memory']],
      BACKENDS: [
        { id: 'k8s', name: 'Kubernetes', where: 'EKS acme-prod · us-east-1', types: ['m7i.2xlarge', 'm7i.4xlarge'], image: 'ratchet-bench-2026.11' },
        { id: 'aws', name: 'AWS VMs', where: 'acme-prod · us-east-1', types: ['c7i.2xlarge', 'c7i.4xlarge'], image: 'ratchet-bench-2026.11' },
        { id: 'gcp', name: 'GCP VMs', where: 'acme-staging · us-central1', types: ['n2-standard-8', 'n2-standard-16'], image: 'ratchet-bench-gcp-2026.11' },
        { id: 'azure', name: 'Azure VMs', where: 'acme-staging · eastus', types: ['D8s v5', 'D16s v5'], image: 'ratchet-bench-az-2026.11' }
      ],
      SECRETS: ['AWS Secrets Manager', 'Google Secret Manager', 'Azure Key Vault', 'HashiCorp Vault'],
      SCOPE: {
        sf: ['A read-only role on the views you list. Holdout tables are not granted.', 'Its own read-only role that also covers the holdout tables.'],
        dbx: ['A read-only token for a service principal with SELECT on the schemas you list. The holdout schema is excluded.', 'Its own token that also covers the holdout schema.'],
        ddb: ['A read-only IAM policy on the tables you list. The holdout table is excluded.', 'Its own policy that adds the holdout table.'],
        bq: ['A read-only service account on authorized views. Holdout views are not authorized.', 'Its own service account that also reads the holdout views.'],
        pg: ['A read-only role on a replica. Holdout tables are not granted.', 'Its own read-only role that also covers the holdout tables.'],
        obj: ['A read-only policy on the prefixes you list. The holdout prefix is excluded.', 'Its own policy that adds the holdout prefix.'],
        gh: ['A short-lived, read-only token scoped to one repository, with holdout paths excluded. Opening a pull request is a separate, audited action.', 'Its own short-lived token that includes the holdout paths.'],
        gl: ['A short-lived, read-only token scoped to one project, with holdout paths excluded. Opening a merge request is a separate, audited action.', 'Its own short-lived token that includes the holdout paths.'],
        mcp: ['A per-run token limited to the read-only tools you allow.', 'Its own per-run token.'],
        rh: ['Past trials of the same experiment, read-only.', 'Not used.'],
        any: ['A read-only key limited to the services and queries you list. Holdout data is excluded.', 'Its own read-only key that also covers the holdout data.']
      },
      MONTHS: ['Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov'],
      SUGG: [['Lower checkout p95 latency', 'Checkout latency'], ['Cut forecast error', 'Forecast error'], ['Speed up CLI cold start', 'CLI cold start'], ['Shrink the bundle', 'Bundle size']],
      CATALOG: [
        G('bq', 'BigQuery', 'Tables and views from your warehouse, read where they live', 'Data', 'R', 21, { def: 'acme-analytics', gl: 'g-db', mg: 'BQ' }),
        G('pg', 'Postgres', 'Read-only queries against a database replica', 'Data', 'R', 17, { def: 'orders-db', gl: 'g-db', mg: 'PG' }),
        G('obj', 'Amazon S3', 'Files and exports in a bucket, versioned by prefix', 'Data', 'R', 9, { def: 's3://acme-data', gl: 'g-box', mg: 'OS' }),
        G('gl', 'GitLab', 'Repositories and merge requests', 'Code', 'R', 7, { def: 'acme/platform', gl: 'g-git', mg: 'GL' }),
        G('bb', 'Bitbucket', 'Repositories and pull requests in your account', 'Code', 'R', 3, { def: 'acme/services', gl: 'g-git', mg: 'BB' }),
        G('gf', 'Grafana', 'Dashboards and service metrics', 'Monitoring', 'C', 4, { def: 'metrics', gl: 'g-chart', mg: 'GF' }),
        G('prom', 'Prometheus', 'Time series from your clusters, read-only', 'Monitoring', 'C', 6, { def: 'prod-cluster', gl: 'g-chart', mg: 'PM' }),
        G('sen', 'Sentry', 'Errors and release health, grouped by issue', 'Monitoring', 'R', 5, { def: 'acme-web', gl: 'g-gauge', mg: 'SN' }),
        G('lin', 'Linear', 'Issues and project notes agents can read for context', 'Code', 'C', 2, { def: 'Platform team', gl: 'g-file', mg: 'LN' }),
        G('mcp', 'Docs search', 'An MCP server over your internal documentation', 'Code', 'C', 3, { def: 'docs.acme.internal', gl: 'g-code', mg: 'DS' }),
        G('dbx', 'Databricks', 'Tables and volumes in your lakehouse, read through Unity Catalog', 'Data', 'R', 8, { def: 'acme-lakehouse', gl: 'g-db', mg: 'DB' }),
        G('ddb', 'DynamoDB', 'Items in your tables, read by key and query', 'Data', 'R', 5, { def: 'orders', gl: 'g-db', mg: 'DY' }),
        G('gh', 'GitHub', 'Repositories, pull requests and design notes agents read', 'Code', 'R', 14, { def: 'acme/checkout', gl: 'g-git', mg: 'GH' }),
        G('sf', 'Snowflake', 'Warehouse tables and the holdout datasets', 'Data', 'R', 9, { def: 'ANALYTICS', gl: 'g-db', mg: 'SF' }),
        G('dd', 'Datadog', 'Production traces, query logs and service metrics', 'Monitoring', 'R', 11, { def: 'production', gl: 'g-chart', mg: 'DD' }),
      ],
      EVCAT: [
        G('http', 'HTTP latency', 'Request latency percentiles from a replayed HTTP workload', 'Speed', 'R', 9, { metric: 'HTTP p95 latency', fmt: ['', ' ms', 0], dir: 'lower', base: 230, n: 2.4, cmd: 'k6 run bench/http.js --vus 150', path: 'services/api', ho: 'ho_checkout', kw: 'http latency request p95 api', gl: 'g-gauge' }),
        G('mstart', 'App start-up', 'Cold start time of the mobile app on a reference device', 'Speed', 'C', 3, { metric: 'Cold start time', fmt: ['', ' ms', 0], dir: 'lower', base: 1380, n: 3.5, cmd: 'maestro test perf/cold-start.yaml', path: 'mobile/app', ho: 'ho_cli', kw: 'mobile app start startup cold', gl: 'g-gauge' }),
        G('mem', 'Memory', 'Peak resident memory during a fixed workload', 'Size', 'R', 5, { metric: 'Peak memory', fmt: ['', ' MB', 0], dir: 'lower', base: 512, n: 1.5, cmd: 'node bench/memory.js', path: 'src', ho: 'ho_checkout', kw: 'memory ram heap', gl: 'g-box' }),
        G('tests', 'Test suite time', 'Wall time of the full test suite on a fresh machine', 'Speed', 'R', 6, { metric: 'Suite duration', fmt: ['', ' s', 0], dir: 'lower', base: 640, n: 3.8, cmd: 'pnpm test --run', path: 'tests', ho: 'ho_checkout', kw: 'test suite ci time', gl: 'g-gauge' }),
        G('llm', 'LLM eval', 'Scores model answers against a labelled set', 'Quality', 'C', 2, { metric: 'Answer score', fmt: ['', '', 2], dir: 'higher', base: 0.72, n: 1.9, cmd: 'python -m evals.run --set answers-v2', path: 'prompts', ho: 'ho_rel', kw: 'llm model answer score prompt', gl: 'g-spark' }),
        G('web', 'Page load', 'Largest contentful paint on a throttled browser', 'Speed', 'R', 7, { metric: 'Largest contentful paint', fmt: ['', ' ms', 0], dir: 'lower', base: 2100, n: 2.8, cmd: 'lhci autorun', path: 'web/src', ho: 'ho_build', kw: 'page load lcp web vitals', gl: 'g-gauge' }),
        G('docker', 'Image size', 'Compressed size of the container image after a clean build', 'Size', 'R', 4, { metric: 'Image size', fmt: ['', ' MB', 0], dir: 'lower', base: 380, n: 0.5, cmd: 'docker build . && ./size.sh', path: 'Dockerfile', ho: 'ho_build', kw: 'docker container image size', gl: 'g-box' }),
        G('sql', 'Query time', 'Runtime of a fixed set of SQL queries on a warehouse replica', 'Speed', 'C', 3, { metric: 'Query time, p50', fmt: ['', ' s', 1], dir: 'lower', base: 8.4, n: 3.1, cmd: 'python -m sqlbench run', path: 'sql', ho: 'ho_pipe', kw: 'sql query warehouse time', gl: 'g-db' }),
        G('cost', 'Cloud cost', 'Daily cost of a service, read from billing exports', 'Cost', 'C', 3, { metric: 'Cost per day', fmt: ['$', '', 0], dir: 'lower', base: 940, n: 2.0, cmd: 'python -m billing.report', path: 'infra', ho: 'ho_pipe', kw: 'cloud cost billing spend', gl: 'g-cloud' }),
        G('lint', 'Type coverage', 'Share of code covered by static types', 'Quality', 'C', 2, { metric: 'Typed share', fmt: ['', '%', 1], dir: 'higher', base: 71.0, n: 0.8, cmd: 'tsc --noEmit --listFiles', path: 'src', ho: 'ho_build', kw: 'type coverage typescript lint', gl: 'g-code' }),
        G('bt', 'Build time', 'Wall time of a clean build from an empty cache', 'Speed', 'R', 8, { metric: 'Build duration', fmt: ['', ' s', 0], dir: 'lower', base: 214, n: 2.4, cmd: 'pnpm build --force', path: 'build', ho: 'ho_build', kw: 'build time compile bundle', gl: 'g-gauge' }),
        G('rpc', 'gRPC latency', 'Request latency percentiles for service to service calls', 'Speed', 'C', 4, { metric: 'gRPC p99 latency', fmt: ['', ' ms', 0], dir: 'lower', base: 38, n: 3.2, cmd: 'ghz --config bench/rpc.json', path: 'services', ho: 'ho_qry', kw: 'grpc rpc latency p99 service', gl: 'g-gauge' }),
        G('flaky', 'Flaky tests', 'Share of test runs that fail then pass on retry', 'Quality', 'R', 4, { metric: 'Flake rate', fmt: ['', '%', 2], dir: 'lower', base: 3.4, n: 4.0, cmd: 'pnpm test --repeat 20', path: 'tests', ho: 'ho_checkout', kw: 'flaky test retry', gl: 'g-gauge' })
      ],
      AGENTCAT: [
        G('gemini', 'Gemini CLI', 'Terminal coding agent that works with Google models', 'Hosted', 'R', 4, { provider: 'Google', model: 'gemini-2.5-pro', via: 'Key in your secret manager', gl: 'g-spark' }),
        G('aider', 'Aider', 'Edits files in your repository and commits each change with a message', 'Open source', 'C', 3, { provider: 'Bring your own model', model: 'configured per run', via: 'Acme gateway', gl: 'g-term' }),
        G('openhands', 'OpenHands', 'Plans a change, edits files and runs the tests before proposing it', 'Open source', 'C', 2, { provider: 'Bring your own model', model: 'configured per run', via: 'Acme gateway', gl: 'g-bot' }),
        G('goose', 'Goose', 'Extensible agent that works through tasks with your own tools', 'Open source', 'C', 2, { provider: 'Bring your own model', model: 'configured per run', via: 'Acme gateway', gl: 'g-bot' }),
        G('cline', 'Cline', 'Plans and edits step by step, asking for approval on risky changes', 'Open source', 'C', 2, { provider: 'Bring your own model', model: 'configured per run', via: 'Acme gateway', gl: 'g-code' }),
        G('amp', 'Amp', 'Terminal agent tuned for large codebases and long tasks', 'Hosted', 'R', 3, { provider: 'Sourcegraph', model: 'amp-default', via: 'Key in your secret manager', gl: 'g-spark' }),
        G('qwen', 'Qwen Code', 'Coding agent built on open Qwen models you can host yourself', 'Open source', 'C', 1, { provider: 'Alibaba', model: 'qwen3-coder', via: 'Acme gateway', gl: 'g-term' }),
        G('opencode', 'OpenCode', 'Provider-agnostic terminal agent with a plain config file', 'Open source', 'C', 2, { provider: 'Bring your own model', model: 'configured per run', via: 'Acme gateway', gl: 'g-term' }),
        G('crush', 'Crush', 'Terminal agent with a focus on fast, small diffs', 'Hosted', 'C', 1, { provider: 'Bring your own model', model: 'configured per run', via: 'Acme gateway', gl: 'g-bot' }),
        G('kilo', 'Kilo Code', 'Agent that splits a goal into tasks and works through them', 'Open source', 'C', 1, { provider: 'Bring your own model', model: 'configured per run', via: 'Acme gateway', gl: 'g-bot' }),
        G('copilot', 'Copilot CLI', 'GitHub terminal agent that proposes changes as a branch', 'Hosted', 'R', 3, { provider: 'GitHub', model: 'default', via: 'GitHub organization acme', gl: 'g-git' })
      ],
      CATS: { eval: ['All', 'Speed', 'Size', 'Quality', 'Cost'], skill: ['All', 'Speed', 'Data', 'Size', 'Quality', 'Cost', 'Conventions'], conn: ['All', 'Data', 'Code', 'Monitoring'], agent: ['All', 'Hosted', 'Open source'], plugin: ['All', 'Performance', 'Quality', 'Cost'] },
      TABS: [['eval', 'Evaluations'], ['skill', 'Skills'], ['conn', 'Connectors'], ['agent', 'Agents'], ['plugin', 'Plugins']],
      MORE: { eval: 'New evaluations', skill: 'New skills', conn: 'Top connectors', agent: 'More agents', plugin: 'New plugins' },
      FEAT: { eval: 'web', skill: 'bundle', plugin: 'webperf' },
      FEATTXT: { eval: 'Time a page load on a throttled browser and see which change makes it faster.', skill: 'Route-level splitting and dependency swaps that cut gzip size without breaking routes.', plugin: 'Page load, HTTP latency and build size, with the know-how to improve them.' },
      SLIDES: [{ t: 'Data warehouses', d: 'Agents read tables and views where they live, through a read-only credential. Nothing is copied.', cat: 'Data', tint: 'var(--infra)', ids: ['sf', 'dbx', 'bq', 'pg'] }, { t: 'Code hosts', d: 'Repositories and review threads that agents read before they propose a change.', cat: 'Code', tint: 'var(--accent)', ids: ['gh', 'gl', 'bb', 'lin'] }, { t: 'Storage and tables', d: 'Files in a bucket and items in a table, read through a policy that names what is allowed.', cat: 'Data', tint: 'var(--good)', ids: ['obj', 'ddb'] }, { t: 'Monitoring', d: 'Traces, metrics and errors from production, read-only.', cat: 'Monitoring', tint: 'var(--warn)', ids: ['dd', 'gf', 'prom', 'sen'] }],
      PSUGG: [['webperf', 'web', 'Add the Web performance plugin to measure page load'], ['llmq', 'llm', 'Add the LLM quality plugin to score model answers'], ['mobile', 'mstart', 'Add the Mobile start-up plugin to time cold starts']],
      NOUN: { eval: 'evaluations', skill: 'skills', conn: 'connectors', agent: 'agents', plugin: 'plugins' },
      TABN: { eval: 'Evaluations', skill: 'Skills', conn: 'Connectors', agent: 'Agents', plugin: 'Plugins' },
      SKILLS, PLUGINS
    };
  }
  theme() {
    if (this.state.theme) return this.state.theme;
    const p = this.props.theme ?? 'system';
    if (p === 'dark' || p === 'light') return p;
    try { return window.matchMedia('(prefers-color-scheme: light)').matches ? 'light' : 'dark'; } catch (e) { return 'dark'; }
  }
  now() {
    const d = new Date();
    return String(d.getHours()).padStart(2, '0') + ':' + String(d.getMinutes()).padStart(2, '0');
  }
  log(action, resource, who) {
    this.setState((s) => ({ audit: [{ time: this.now(), who: who || 'Priya Raman', action, resource }, ...s.audit] }));
  }
