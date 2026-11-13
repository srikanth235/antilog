  // ---- helpers
  initials(n) { const w = n.split(' '); return w.length > 1 ? w.map((x) => x.charAt(0)).join('').slice(0, 2) : n.charAt(0) + n.charAt(n.length - 1); }
  cap1(t) { return t.charAt(0).toUpperCase() + t.slice(1); }
  low(t) { return /^[A-Z][A-Z]/.test(t) ? t : t.charAt(0).toLowerCase() + t.slice(1); }
  money(n) { return '$' + Number(n).toLocaleString('en-US'); }
  fv(ev, v) { const [p, sf, dec] = ev.fmt; return p + Number(v).toLocaleString('en-US', { minimumFractionDigits: dec, maximumFractionDigits: dec }) + sf; }
  pc(ev, v) { return ((v - ev.base) / ev.base) * 100; }
  pt(p) { const a = Math.abs(p).toFixed(1); return (p > 0.049 ? '+' : p < -0.049 ? '−' : '') + a + '%'; }
  fav(ev, p) { return ev.dir === 'lower' ? -p : p; }
  cls(ev, v, h) {
    const f = this.fav(ev, this.pc(ev, v));
    if (f > 2 * ev.n) return h != null && this.fav(ev, this.pc(ev, h)) <= 2 * ev.n ? 'holdout' : 'accepted';
    return f < -ev.n ? 'worse' : 'noise';
  }
  fmtMin(m) { return m >= 60 ? Math.floor(m / 60) + ' h ' + (m % 60) + ' m' : m + ' m'; }
  spend(o) { return o.fixed !== null ? o.fixed : Math.round(o.cap * this.K.FR[o.step]); }
  isDone(o) { return ['Win', 'Pending pull request', 'Merged', 'No win'].includes(o.status); }
  isWin(o) { return ['Win', 'Pending pull request', 'Merged'].includes(o.status); }
  evOf(s, id) { return s.evals.find((e) => e.id === id) || s.evals[0]; }
  agName(s, id) { return (s.agentList.find((a) => a.id === id) || {}).name || this.K.AGN[id] || id; }
  an(o, s) {
    const ev = { ...this.evOf(s, o.ev), dir: o.dir || this.evOf(s, o.ev).dir };
    const TT = o.step >= 4 ? o.T : [];
    const mc = o.step >= 6 ? TT.length : (o.step === 5 ? Math.min(1, TT.length) : 0);
    const cur = TT.map((t, i) => {
      const idea = ev.ideas[t.i % ev.ideas.length];
      const mes = i < mc;
      const base0 = mes ? this.cls(ev, t.v, t.h) : (o.status === 'Stopped' ? 'disc' : 'measuring');
      const out = base0 === 'accepted' && o.step < 6 ? 'pend' : base0;
      return { ...t, i0: i, mes, out, base0, p: mes ? this.pc(ev, t.v) : null, agent: this.agName(s, t.a), run: o.runs, key: o.id + ':' + i, text: idea[0], files: idea[1], add: idea[2], del: idea[3], diff: idea[4], tm: [6, 9, 12][i % 3] };
    });
    const past = o.hist.map(([run, a, text, v]) => ({ run, agent: this.agName(s, a), text, v, mes: true, out: this.cls(ev, v, null), p: this.pc(ev, v), key: null }));
    const seq = [...past, ...cur];
    let best = null;
    seq.forEach((t) => { if (t.mes && t.out !== 'holdout' && (best === null || this.fav(ev, t.p) > this.fav(ev, best.p))) best = t; });
    const win = cur.filter((t) => t.mes && t.out === 'accepted').sort((a, b) => this.fav(ev, b.p) - this.fav(ev, a.p))[0] || null;
    return { ev, cur, past, seq, best, win, mc };
  }
  evMatch(s, text) {
    const words = (text || '').toLowerCase().match(/[a-z0-9]+/g) || [];
    let best = null, bs = 0;
    s.evals.forEach((e) => { const k = e.kw.split(' '); let sc = 0; words.forEach((w) => { if (k.includes(w)) sc++; }); if (sc > bs) { bs = sc; best = e; } });
    return best || s.evals[0];
  }
  nameFrom(text) {
    let t = (text || '').trim().replace(/[.!?]+$/, '');
    t = t.replace(/^(please\s+)?(lower|reduce|cut|speed up|shrink|improve|increase|raise|boost|decrease|optimi[sz]e|make)\s+(the\s+|our\s+)?/i, '');
    t = t.split(/\s+(without|while|so that|by|and)\s+/i)[0];
    const w = t.split(/\s+/).slice(0, 5).join(' ');
    return w ? w.charAt(0).toUpperCase() + w.slice(1) : '';
  }
  nextRun(x, a, label) {
    const hist = [...x.hist, ...a.cur.filter((t) => t.mes).map((t) => [x.runs, t.a, t.text, t.v])];
    return { ...x, hist, pr: [...x.pr, label], runs: x.runs + 1, status: 'Draft', approved: false, step: 0, T: [], ps: x.ps + this.spend(x), fixed: null, act: 'Just now' };
  }
  genT(o, s) {
    const ev = { ...this.evOf(s, o.ev), dir: o.dir || this.evOf(s, o.ev).dir };
    const odd = o.runs % 2 === 1;
    const pat = odd ? [12.5, 0.9, -7.4] : [1.2, 8.8, -3.0];
    const hf = odd ? [0.95, 0.5, 0] : [0.5, 0.1, 0];
    return pat.slice(0, Math.min(o.agents.length, 3)).map((f, i) => {
      const rnd = (x) => Number(x.toFixed(ev.fmt[2]));
      const m = (k) => (ev.dir === 'lower' ? ev.base * (1 - k / 100) : ev.base * (1 + k / 100));
      return { a: o.agents[i], i, v: rnd(m(f)), h: rnd(m(f * hf[i])), e: ['0.3%', '0.2%', '1.6%'][i], c: ['+$0.01', '$0.00', '+$0.02'][i], f: i === 2 ? 'e' : '' };
    });
  }
  declare(o, a, s) {
    const K = this.K, ev = a.ev;
    const lim = K.LIMITS.filter(([k]) => o.lim[k]).map(([, l]) => l.toLowerCase());
    const pk = s.datasets.filter((x) => o.ds.includes(x.id));
    const hold = s.datasets.find((x) => x.id === ev.ho && x.marking === 'Holdout') || s.datasets.find((x) => x.marking === 'Holdout');
    const cl = K.CLOUDS.find((c) => c[0] === o.env) || K.CLOUDS[0];
    const ag = o.agents.length + (o.agents.length === 1 ? ' agent' : ' agents');
    const sk = (o.skills || []).map((id) => s.skills.find((k) => k.id === id)).filter(Boolean);
    return {
      hold, pk, sk,
      lines: [
        { k: 'Experiment', v: o.name + ', scored by ' + ev.name, c: K.COL.opt },
        { k: 'Primary metric', v: ev.metric + ' · ' + ev.dir + ' is better · baseline ' + this.fv(ev, ev.base) + ', noise ±' + ev.n + '%', c: K.COL.opt },
        { k: 'Guardrail metrics', v: lim.length ? lim.join(', ') : 'none', c: K.COL.opt },
        { k: 'Editable paths', v: o.path || ev.path, c: K.COL.opt },
        { k: 'Data', v: (pk.length ? pk.map((x) => x.name).join(', ') : 'no datasets') + (hold ? '. ' + hold.name + ' is restricted to the evaluator.' : '. No dataset is marked Holdout.'), c: K.COL.ctx },
        { k: 'Skills', v: sk.length ? sk.map((k) => k.name).join(', ') + '. Agents read these. The evaluator does not.' : 'No skills selected', c: K.COL.ctx },
        { k: 'Environment', v: cl[2] + ', ' + ag + ', ' + this.money(o.cap) + ' budget, removed after the last run', c: K.COL.inf }
      ],
      chips: ['Data · ' + pk.length + (pk.length === 1 ? ' dataset' : ' datasets'), 'Skills · ' + sk.length, 'Environment · ' + cl[1], 'Agents · ' + o.agents.length, 'Budget · ' + this.money(o.cap)],
      summary: ev.metric + ', ' + pk.length + (pk.length === 1 ? ' dataset, ' : ' datasets, ') + sk.length + (sk.length === 1 ? ' skill, ' : ' skills, ') + cl[1] + ', ' + ag + ', ' + this.money(o.cap)
    };
  }
  renderVals() {
    const s = this.state, K = this.K;
    const theme = this.theme();
    const money = (n) => this.money(n);
    const upd = (id, patch) => this.setState((st) => ({ opts: st.opts.map((x) => (x.id === id ? { ...x, ...patch } : x)) }));
    const setD = (patch) => this.setState((st) => ({ d: { ...st.d, ...patch } }));
    const seg = (list, cur, pick) => list.map(([v, l]) => ({ label: l || v, cls: v === cur ? 'seg on' : 'seg', pick: () => pick(v) }));
    const digits = (e) => String(e.target.value).replace(/\D/g, '').slice(0, 7);
    const is = (id) => s.screen === id;
    const menu = s.menu;
    const mopen = (id) => menu === id;
    const tmenu = (id) => () => this.setState({ menu: menu === id ? null : id });
    const gl = (id) => id;
    const st0 = (o) => K.STAT[o.status];
    const stOf = (o) => { const t = st0(o); return { stLabel: t[0], stColor: t[2], stCls: t[3] + ' gi ' + t[1] }; };
    const selOpt = s.opts.find((o) => o.id === s.selId) || s.opts[0];
    const cloudLabel = (k) => K.CLOUDS.find((c) => c[0] === k)[1];
    const roleOf = (p) => (K.PROJECTS.find((x) => x[0] === p) || [0, 'Viewer'])[1];
    const evAll = (o) => this.an(o, s);

    const goTo = (screen, extra) => () => this.setState({ screen, menu: null, confirmStop: false, renaming: false, ...(extra || {}) });
    const openOpt = (o) => () => this.setState({ screen: 'thread', selId: o.id, lastProject: o.project, projSel: o.project, menu: null, confirmStop: false, renaming: false, hl: null, pal: false });
    const custReset = { cQuery: '', cCat: 'All', cAuth: 'All', cAdded: 'All', cNotice: '', form: null, justAdded: {}, plRemove: null, skSec: 0 };
    const goCust = (patch) => () => this.setState({ screen: 'customize', menu: null, ...custReset, ...patch });

    // ---------- composer draft
    const d = s.d;
    const dEv = d.ev ? this.evOf(s, d.ev) : this.evMatch(s, d.text);
    const dProj = d.project || s.lastProject;
    const dDir = d.dir || dEv.dir;
    const dName = d.name !== null && d.name !== undefined && d.name !== '' ? d.name : (this.nameFrom(d.text) || 'New experiment');
    const dPath = d.path !== null && d.path !== undefined ? d.path : dEv.path;
    const dHold = s.datasets.find((x) => x.id === dEv.ho && x.marking === 'Holdout') || s.datasets.find((x) => x.marking === 'Holdout');
    const selIds = s.datasets.filter((x) => x.marking !== 'Holdout' && s.newSel[x.id]).map((x) => x.id);
    const canDraft = !!(d.text || '').trim() && roleOf(dProj) !== 'Viewer';
    const enabled = s.agentList.filter((a) => s.agents[a.id]).map((a) => a.id);
    const onSk = s.skills.filter((k) => !s.skillsOff[dProj + ':' + k.id]);
    const autoSk = onSk.filter((k) => (k.ev || []).includes('*') || (k.ev || []).includes(dEv.id));
    const selSk = d.skills ? onSk.filter((k) => d.skills.includes(k.id)) : (autoSk.length ? autoSk : onSk);
    const selSkIds = selSk.map((k) => k.id);

    // ---------- sidebar
    const sessRow = (o) => {
      const t = st0(o);
      return {
        name: o.name, cls: is('thread') && s.selId === o.id ? 'rec on' : 'rec', open: openOpt(o),
        hasCnt: o.runs > 1, cnt: String(o.runs), cntTip: o.runs + ' runs',
        stCls: t[3] + ' gi ' + t[1], stTip: t[0], stColor: t[2],
        pinTip: (o.pinned ? 'Unpin ' : 'Pin ') + o.name, pin: () => upd(o.id, { pinned: !o.pinned }), pinned: o.pinned
      };
    };
    const pinned = s.opts.filter((o) => o.pinned).map(sessRow);
    const groups = K.PROJECTS.map(([n]) => {
      const items = s.opts.filter((o) => o.project === n && !o.pinned).map(sessRow);
      const open = !s.collapsed[n];
      return {
        name: n, open, rot: open ? 'rotate(90deg)' : 'rotate(0deg)', show: true, items: open ? items : [],
        count: items.length, canNew: roleOf(n) !== 'Viewer', plusTip: 'New experiment in ' + n,
        toggle: () => this.setState((st) => ({ collapsed: { ...st.collapsed, [n]: !st.collapsed[n] } })),
        plus: () => this.setState({ screen: 'new', menu: null, editId: null, d: { ...s.d, text: '', name: null, ev: null, path: null, dir: null, project: n, skills: null }, needText: false })
      };
    });
    const sideActive = { projects: 'projects', project: 'projects', value: 'value', customize: 'customize', detail: 'customize' }[s.screen];
    const nav = (id) => (sideActive === id ? 'nav on' : 'nav');

    // ---------- breadcrumb
    const settingsTabs = [['general', 'General'], ['notes', 'Notes'], ['budgets', 'Budgets'], ['members', 'Users and groups'], ['signin', 'Single sign-on'], ['tokens', 'Access tokens'], ['audit', 'Audit logs']];
    const tabLabel = (settingsTabs.find((t) => t[0] === s.settingsTab) || settingsTabs[0])[1];
    const detKind = s.detail ? s.detail.kind : 'eval';
    const detEval = detKind === 'eval' ? s.evals.find((e) => e.id === s.detail.id) : null;
    const detAgent = detKind === 'agent' ? s.agentList.find((a) => a.id === s.detail.id) : null;
    const detConn = detKind === 'conn' ? s.connectors.find((c) => c.id === s.detail.id) : null;
    const detSkill = detKind === 'skill' ? s.skills.find((k) => k.id === s.detail.id) || K.SKILLS.find((k) => k.id === s.detail.id) : null;
    const detPlugin = detKind === 'plugin' ? s.plugins.find((k) => k.id === s.detail.id) || K.PLUGINS.find((k) => k.id === s.detail.id) : null;
    const detName = (detEval || detAgent || detConn || detSkill || detPlugin || { name: '' }).name;
    const cr = (label, go) => ({ label, go, link: !!go });
    const crumbs = ({
      new: [cr('New experiment')],
      thread: [cr(selOpt.project, goTo('project', { projSel: selOpt.project })), cr(selOpt.name)],
      projects: [cr('Projects')],
      project: [cr('Projects', goTo('projects')), cr(s.projSel)],
      value: [cr('Dashboard')],
      customize: [cr('Customize')],
      detail: [cr('Customize', goCust({})), cr(K.TABN[detKind === 'conn' ? 'conn' : detKind], goCust({ cTab: detKind, cView: 'yours' })), cr(detName)],
      history: [cr('Run history')],
      settings: [cr('Settings'), cr(tabLabel)]
    })[s.screen] || [cr('Ratchet')];
    const crumbRows = crumbs.map((c, i) => ({ ...c, txt: !c.link, sep: i > 0 }));

    // ---------- thread
    const o = selOpt;
    const a = evAll(o);
    const ev = a.ev;
    const dc = this.declare(o, a, s);
    const live = o.status === 'Running';
    const done = this.isDone(o);
    const stopped = o.status === 'Stopped';
    const winT = a.win;
    const oSpend = this.spend(o);
    const oNotes = s.notes.filter((n) => n.opt === o.name && n.run === o.runs);
    const exKey = (k) => k + ':' + o.id;
    const toggleEx = (k) => () => this.setState((st) => ({ ex: { ...st.ex, [k]: !st.ex[k] } }));
    const approvedAt = o.approved;
    const detOpen = !approvedAt || !!s.ex[exKey('det')];
    const showProg = approvedAt && o.step >= 1;
    const curStep = Math.min(o.step, 8);
    const trialCount = a.seq.length;
    const nm = o.agents.length;
    const measuring = a.cur.filter((t) => !t.mes).length;
    const stepText = [
      '', 'Starting the run.', 'A runner is starting in ' + cloudLabel(o.env) + '.', 'Agents are getting read access to ' + dc.pk.length + (dc.pk.length === 1 ? ' dataset.' : ' datasets.'),
      nm + (nm === 1 ? ' agent is' : ' agents are') + ' proposing changes.', 'The evaluator is measuring ' + measuring + (measuring === 1 ? ' trial' : ' trials') + ' on a fresh host.',
      'Checking each result against the acceptance criteria.', 'Logging the run to history and preparing the pull request.', 'Terminating hosts and verifying clean-up.'
    ][curStep] || '';
    const nodes = [];
    K.STEPS.forEach(([c, who, what], i) => {
      const n = i + 1;
      const state = o.step >= 8 ? 'done' : (n < o.step ? 'done' : (n === o.step ? (live ? 'cur' : (stopped ? 'stop' : 'done')) : 'up'));
      nodes.push({ cls: 'nd ' + state, title: n + '. ' + who + ' · ' + what, color: K.COL[c], ln: false });
      nodes.push({ cls: 'ln ' + (n < o.step || o.step >= 8 ? 'done' : ''), ln: true });
    });
    nodes.pop();
    const hl = s.hl;
    const trials = a.cur.map((t) => {
      const O = K.OUT[t.out];
      const holdSh = t.h != null && t.mes && t.base0 !== 'noise' && t.base0 !== 'worse';
      const holdPass = holdSh && this.fav(ev, this.pc(ev, t.h)) > 2 * ev.n;
      const lims = K.LIMITS.filter(([k]) => o.lim[k]).map(([k, l]) => {
        const fail = (t.f || '').includes(k.charAt(0));
        const val = k === 'error' ? (t.e || '0.2%') : k === 'cost' ? (t.c || '$0.00') : (t.m || '+4 MB');
        return { k: l, v: val, ok: !fail, tag: fail ? 'Fail' : 'Pass', color: fail ? 'var(--bad)' : 'var(--good)' };
      });
      const open = !!s.ex['tr:' + t.key];
      return {
        dom: 'tr-' + t.key.replace(':', '-'), agent: t.agent, init: this.initials(t.agent), text: t.text, files: t.files + (t.files === 1 ? ' file' : ' files'), add: '+' + t.add, del: '−' + t.del,
        value: t.mes ? this.fv(ev, ev.base) + ' → ' + this.fv(ev, t.v) : (t.out === 'disc' ? 'Not measured' : 'Measuring on a fresh host'),
        mes: t.mes, delta: t.mes ? this.pt(t.p) : '', deltaColor: t.mes ? (O[1] === 'var(--ink3)' ? 'var(--ink2)' : O[1]) : 'var(--ink3)',
        out: O[0], outColor: O[1], outCls: t.mes ? 'tg' : 'tg hollow', tm: t.mes ? t.tm + ' m' : '',
        open, rot: open ? 'rotate(90deg)' : 'rotate(0deg)', hl: hl === t.key, rowCls: hl === t.key ? 'trow hl' : 'trow',
        toggle: () => this.setState((st) => ({ ex: { ...st.ex, ['tr:' + t.key]: !st.ex['tr:' + t.key] } })),
        enter: () => this.setState({ hl: t.key }), leave: () => this.setState((st) => (st.hl === t.key ? { hl: null } : null)),
        diff: t.diff.map((l) => ({ t: l, cls: l.charAt(0) === '+' ? 'df add' : l.charAt(0) === '-' ? 'df del' : l.charAt(0) === '@' ? 'df hd' : 'df' })),
        lims, showLims: t.mes, hasHold: holdSh, holdTxt: holdSh ? 'Holdout ' + this.fv(ev, t.h) + ' against a baseline of ' + this.fv(ev, ev.base) + '. ' + (holdPass ? 'Passed.' : 'Did not hold up. Failed.') : '',
        holdColor: holdPass ? 'var(--good)' : 'var(--warn)', cite: t.k && (o.skills || []).includes(t.k[0]) ? 'Cited ' + ((s.skills.find((q) => q.id === t.k[0]) || {}).name || 'a skill') + ', ' + t.k[1] + '.' : '', hasCite: !!(t.k && (o.skills || []).includes(t.k[0])),
        pendTxt: t.mes ? '' : (t.out === 'disc' ? 'The run was stopped before this trial was measured.' : 'Waiting for the evaluator.')
      };
    });
    const acceptedTxt = this.pt(winT ? winT.p : 0);
    const pastRuns = o.pr.map((r, i) => {
      const n = o.hist.filter((h) => h[0] === i + 1).length;
      return { t: 'Run ' + (i + 1) + ' · ' + (r === 'w' ? 'a change was accepted' : r === 's' ? 'stopped before a result' : 'no change beat the baseline') + (n ? ' · ' + n + (n === 1 ? ' trial' : ' trials') : '') };
    });
    // chart
    const seq = a.seq;
    const vals = seq.filter((t) => t.mes).map((t) => t.v);
    const band = ev.base * ev.n / 100;
    const lo0 = Math.min(ev.base - band, ...(vals.length ? vals : [ev.base]));
    const hi0 = Math.max(ev.base + band, ...(vals.length ? vals : [ev.base]));
    const padv = (hi0 - lo0) * 0.14 || 1;
    const lo = lo0 - padv, hi = hi0 + padv;
    const Y = (v) => ((hi - v) / (hi - lo)) * 100;
    const N = seq.length;
    const Xp = (i) => (N === 1 ? 50 : 4 + (92 * i) / (N - 1));
    const pct = (x) => x.toFixed(2) + '%';
    const hasChart = N > 0;
    const dots = seq.map((t, i) => {
      const O = K.OUT[t.out] || K.OUT.noise;
      const label = t.agent + ', run ' + t.run + ': ' + (t.mes ? this.fv(ev, t.v) + ' (' + this.pt(t.p) + '), ' + O[0].toLowerCase() : 'measuring');
      return {
        left: pct(Xp(i)), top: pct(Y(t.mes ? t.v : ev.base)), color: O[1], tip: label, hollow: !t.mes, hl: t.key && hl === t.key,
        cls: (t.mes ? 'dot2' : 'dot2 hollow') + (t.key && hl === t.key ? ' hl' : ''),
        enter: () => t.key && this.setState({ hl: t.key }), leave: () => this.setState((st) => (st.hl === t.key ? { hl: null } : null)),
        click: () => {
          if (!t.key) return;
          this.setState((st) => ({ hl: t.key, ex: { ...st.ex, ['tr:' + t.key]: true } }));
          setTimeout(() => { const el = document.getElementById('tr-' + t.key.replace(':', '-')); if (el && el.scrollIntoView) el.scrollIntoView({ block: 'center', behavior: 'smooth' }); }, 60);
        }
      };
    });
    const segs = [];
    let prevI = -1, bestV = null, bestF = null;
    seq.forEach((t, i) => {
      if (!t.mes || t.out === 'holdout') return;
      const f = this.fav(ev, t.p);
      const better = bestF === null || f > bestF;
      if (prevI >= 0) {
        const x0 = Xp(prevI), x1 = Xp(i);
        segs.push({ cls: 'sg h', left: pct(x0), top: pct(Y(bestV)), width: pct(x1 - x0), height: '0' });
        if (better) segs.push({ cls: 'sg v', left: pct(x1), top: pct(Math.min(Y(bestV), Y(t.v))), width: '0', height: pct(Math.abs(Y(bestV) - Y(t.v))) });
      }
      if (better) { bestF = f; bestV = t.v; }
      prevI = i;
    });
    const ticks = [hi0, (hi0 + lo0) / 2, lo0].map((v) => ({ top: pct(Y(v)), t: this.fv(ev, v).replace(/\.0+(?=\D|$)/, '') }));
    const seps = [];
    seq.forEach((t, i) => { if (i === 0) seps.push({ left: '0%', t: 'Run ' + t.run, line: false }); else if (t.run !== seq[i - 1].run) seps.push({ left: pct((Xp(i) + Xp(i - 1)) / 2), t: 'Run ' + t.run, line: true }); });

    // thread notes (with acknowledgement)
    const threadNotes = oNotes.slice().reverse().map((n) => ({
      dom: 'note-' + n.id, cls: s.hlNote === n.id ? 'col g12 flash' : 'col g12', mine: n.author === 'Priya Raman', other: n.author !== 'Priya Raman', who: n.author, init: K.PEOPLE[n.author] || n.author.charAt(0), text: n.text, time: n.time,
      ack: n.ack || (done || stopped ? 'Noted. I will use this in the next run.' : o.status === 'Draft' ? 'Noted. Agents will read this before the first trial.' : 'Noted. Agents will read this before the next trial.')
    }));
    const winPct = winT ? this.pt(winT.p) : '';
    const hitCap = this.spend(o) >= o.cap;
    const resSum = winT ? ev.metric + (ev.dir === 'lower' ? ' fell from ' : ' rose from ') + this.fv(ev, ev.base) + ' to ' + this.fv(ev, winT.v) + ', confirmed on a fresh host and held on data the agents never saw.' : '';
    const wcount = Math.min(o.agents.length, 3);
    const triedList = a.cur.map((t) => ({ agent: t.agent, text: t.text, value: this.fv(ev, t.v) + ' (' + this.pt(t.p) + ')', out: K.OUT[t.out][0], color: K.OUT[t.out][1] }));
    const nHold = a.cur.filter((t) => t.out === 'holdout').length;
    const ag = Math.round(oSpend * 0.49), hosts = Math.round(oSpend * 0.41);
    const kvs = (list) => list.map(([k, v]) => ({ k, v }));
    const hasBest = !!a.best;
    const reqChips = dc.chips.map((t) => ({ t }));
    const optCol = winT ? kvs([['Baseline', this.fv(ev, ev.base) + ' over 9 runs'], ['Best trial', this.fv(ev, winT.v) + ' over 5 runs, same on a second host'], ['Holdout', winT.h != null ? this.fv(ev, winT.h) + ', passed' : 'Passed'], ['Noise', '±' + ev.n + '%']]) : [];
    const ctxCol = kvs([['Read by agents', dc.pk.length ? dc.pk.map((x) => x.name).join(', ') : 'No datasets'], ['Skills read by agents', dc.sk.length ? dc.sk.map((x) => x.name).join(', ') : 'None'], ['Used by evaluator', dc.hold ? dc.hold.name : 'None marked'], ['Notes considered', oNotes.length + (oNotes.length === 1 ? ' note' : ' notes') + ' from people']]);
    const infCol = kvs([['Environment', (K.CLOUDS.find((c) => c[0] === o.env) || K.CLOUDS[0])[2] + ', ' + ['c7i.2xlarge', 'n2-standard-8', 'D8s v5'][Math.max(0, K.CLOUDS.findIndex((c) => c[0] === o.env))]], ['Agents used', String(wcount)], ['Spend', money(oSpend) + ' (agents ' + money(ag) + ', hosts ' + money(hosts) + ')'], ['Clean-up', o.step >= 8 || stopped ? 'Verified' : 'Pending until step 8']]);
    const mach = s.machines.filter((m) => m.opt === o.name);
    const placeholders = { Draft: 'Ask Ratchet to change the draft, e.g. also guard memory use', Running: 'Steer the next trial, e.g. focus on database queries' };
    const ph = placeholders[o.status] || 'Start another run with a note, e.g. try the cache layer';
    const plan = K.STEPS.map(([c, who, what], i) => ({ n: i + 1, who, what, color: K.COL[c] }));
    const prog = K.STEPS.slice(0, o.step).map(([c, who, what], i) => {
      const n = i + 1, now = n === o.step && o.step < 8;
      return { n, who, what, color: K.COL[c], tag: now ? (live ? 'current' : stopped ? 'stopped' : 'done') : 'done', cur: now && live };
    });
    const askBase = { Draft: 'Here is the draft. I will measure ' + this.low(ev.metric) + ' on ' + ev.name + ' and accept a change only if it beats twice the noise and passes the holdout.' };
    const sayDraft = o.runs > 1 ? 'Here is the draft for run ' + o.runs + '. ' + askBase.Draft.replace('Here is the draft. ', '') : askBase.Draft;
    const resSay = winT ? 'One change beat the baseline by ' + winPct.replace('+', '').replace('−', '') + ' and held up on the holdout. Here is what it did.' : 'No change met the acceptance criteria this run. Here is what was tried.';
    const sendThread = () => {
      const text = (s.threadDraft || '').trim();
      if (!text) return;
      const id = 'nn' + Date.now();
      if (o.status === 'Draft') {
        const mem = /memory/i.test(text), cst = /\bcost\b/i.test(text), err = /error/i.test(text);
        const lim = { ...o.lim, memory: o.lim.memory || mem, cost: o.lim.cost || cst, error: o.lim.error || err };
        const added = mem && !o.lim.memory ? 'memory' : cst && !o.lim.cost ? 'cost' : err && !o.lim.error ? 'error rate' : '';
        const ack = added ? 'Updated the draft. I added ' + added + ' as a guardrail metric, and agents will see your note.' : 'Updated the draft. Agents will read your note before the first trial.';
        this.setState((st) => ({ notes: [{ id, text, author: 'Priya Raman', opt: o.name, time: this.now(), ack, run: o.runs }, ...st.notes], threadDraft: '', opts: st.opts.map((x) => (x.id === o.id ? { ...x, lim } : x)) }));
      } else if (live) {
        this.setState((st) => ({ notes: [{ id, text, author: 'Priya Raman', opt: o.name, time: this.now(), run: o.runs }, ...st.notes], threadDraft: '' }));
      } else {
        const label = this.isWin(o) ? 'w' : stopped ? 's' : 'n';
        const nr = o.runs + 1;
        this.setState((st) => ({
          notes: [{ id, text, author: 'Priya Raman', opt: o.name, time: this.now(), run: nr, ack: 'Started run ' + nr + ' as a draft with your note. Review it, then approve to start.' }, ...st.notes], threadDraft: '',
          opts: st.opts.map((x) => (x.id === o.id ? this.nextRun(x, a, label) : x))
        }));
      }
      this.log('note.send', o.name + ' · note');
    };
    const tOutC = trials.length;

    // ---------- value / projects
    const winsList = s.opts.filter((x) => this.isWin(x) || x.pr.includes('w'));
    const winInfo = (x) => { const ax = evAll(x); const w = ax.win; const p = w ? w.p : (ax.best ? ax.best.p : 0); return { ax, p, val: x.val || Math.max(1000, Math.round((Math.abs(p) * 480) / 100) * 100) }; };
    const totalVal = winsList.reduce((t, x) => t + winInfo(x).val, 0);
    const totalSpend = s.opts.reduce((t, x) => t + this.spend(x) + x.ps, 0);
    const finishedRuns = s.opts.reduce((t, x) => t + x.pr.length + (this.isDone(x) ? 1 : 0), 0);
    const winRuns = s.opts.reduce((t, x) => t + x.pr.filter((r) => r === 'w').length + (this.isWin(x) ? 1 : 0), 0);
    const winN = winRuns;
    const kfmt = (n) => '$' + (n / 1000).toFixed(1) + 'k';
    const sortedWins = winsList.slice().sort((x, y) => x.mo - y.mo);
    let cum = 0;
    const gmax = Math.max(15000, Math.ceil(totalVal / 15000) * 15000);
    const ypos = (v) => (100 - (v / gmax) * 100).toFixed(2) + '%';
    const xpos = (m) => (m * 100 / 5).toFixed(2) + '%';
    const dsegs = [], ddots = [];
    let prevM = 0, prevV = 0;
    sortedWins.forEach((w) => {
      const v = winInfo(w).val, m = w.mo;
      dsegs.push({ cls: 'sg h', left: xpos(prevM), top: ypos(prevV), width: ((m - prevM) * 100 / 5).toFixed(2) + '%', height: '0' });
      dsegs.push({ cls: 'sg v', left: xpos(m), top: ypos(prevV + v), width: '0', height: ((v / gmax) * 100).toFixed(2) + '%' });
      prevM = m; prevV += v;
      ddots.push({ left: xpos(m), top: ypos(prevV), tip: w.name + ' · ' + kfmt(v) + ' per year', open: () => openOpt(w)() });
    });
    dsegs.push({ cls: 'sg h', left: xpos(prevM), top: ypos(prevV), width: ((5 - prevM) * 100 / 5).toFixed(2) + '%', height: '0' });
    const dgrid = [0, 1, 2, 3].map((i) => ({ top: (i * 100 / 3).toFixed(2) + '%', t: i === 3 ? '$0' : '$' + Math.round((gmax * (3 - i)) / 3 / 1000) + 'k' }));
    const metricTxt = (x) => { const ax = evAll(x); return ax.best ? ax.ev.metric + ' · ' + this.fv(ax.ev, ax.ev.base) + ' → ' + this.fv(ax.ev, ax.best.v) + ' (' + this.pt(ax.best.p) + ')' : ax.ev.metric + ' · baseline ' + this.fv(ax.ev, ax.ev.base); };
    const projList = K.PROJECTS.map(([n, role, blurb]) => ({ n, role, blurb, list: s.opts.filter((x) => x.project === n) }));
    const pj = projList.find((p) => p.n === s.projSel) || projList[0];

    // ---------- history
    const optProj = (name) => (s.opts.find((x) => x.name === name) || {}).project;
    const histAll = [];
    s.opts.forEach((x) => {
      const ax = evAll(x);
      [...ax.cur.filter((t) => t.mes).reverse(), ...ax.past.slice().reverse()].forEach((t) => {
        const O = K.OUT[t.out];
        histAll.push({ optId: x.id, opt: x.name, project: x.project, run: t.run, agent: t.agent, tried: t.text, result: this.fv(ax.ev, t.v) + ' (' + this.pt(t.p) + ')', verdict: O[0], color: O[1], open: openOpt(x) });
      });
    });
    const hExpOptions = ['All', ...s.opts.filter((x) => s.histProj === 'All' || x.project === s.histProj).map((x) => x.name)];
    const hFilter = (r) => (s.histProj === 'All' || r.project === s.histProj) && (s.histExp === 'All' || r.opt === s.histExp);
    const machAll = s.machines.map((m) => ({ ...m, project: optProj(m.opt), st: m.gone ? 'Terminated, clean-up verified' : 'Running', color: m.gone ? 'var(--ink3)' : 'var(--good)', open: () => { const x = s.opts.find((q) => q.name === m.opt); if (x) openOpt(x)(); } }));
    const machRows = machAll.filter((m) => (s.histProj === 'All' || m.project === s.histProj) && (s.histExp === 'All' || m.opt === s.histExp));

    // ---------- customize
    const tab = s.cTab;
    const yoursView = s.cView === 'yours';
    const cq = (s.cQuery || '').trim().toLowerCase();
    const authOf = (x) => ({ R: 'Ratchet', C: 'Community', T: 'Your team' })[x.byk || 'T'];
    const byTxt = (x) => ({ R: 'by Ratchet', C: 'Community', T: 'Your team' })[x.byk || 'T'];
    const plural = (n, w) => n + ' ' + w + (n === 1 ? '' : 's');
    const joinAnd = (a) => (a.length < 2 ? a.join('') : a.slice(0, -1).join(', ') + ' and ' + a[a.length - 1]);
    const KN = [['eval', 'evaluation'], ['skill', 'skill'], ['conn', 'connector'], ['agent', 'agent']];
    const plCount = (p) => KN.filter(([k]) => p.items[k].length).map(([k, w]) => plural(p.items[k].length, w)).join(' · ');
    const skUsers = (id) => s.opts.filter((q) => (q.skills || []).includes(id));
    const skOn = (id, proj) => !s.skillsOff[(proj || s.lastProject) + ':' + id];
    const poolOf = { eval: s.evals, skill: s.skills, conn: s.connectors, agent: s.agentList, plugin: s.plugins };
    const catOf = { eval: K.EVCAT, skill: K.SKILLS, conn: K.CATALOG, agent: K.AGENTCAT, plugin: K.PLUGINS };
    const isY = (kind, id) => poolOf[kind].some((q) => q.id === id);
    const resolve = (kind, id) => poolOf[kind].find((q) => q.id === id) || catOf[kind].find((q) => q.id === id);
    const usedN = (kind, x) => (kind === 'skill' && isY('skill', x.id) ? skUsers(x.id).length : x.used || 0);
    const inY = (x) => isY(tab, x.id);
    const matchC = (x) => (!cq || (x.name + ' ' + (x.desc || '') + ' ' + (x.cat || '')).toLowerCase().includes(cq)) && (s.cCat === 'All' || x.cat === s.cCat) && (s.cAuth === 'All' || authOf(x) === s.cAuth) && (yoursView || s.cAdded === 'All' || (s.cAdded === 'Added') === inY(x));
    const sortC = (p, q) => (s.cSort === 'used' ? q._u - p._u || p.name.localeCompare(q.name) : s.cSort === 'new' ? (yoursView ? q._i - p._i : p._i - q._i) : p.name.localeCompare(q.name));
    const glFor = (x, t) => x.gl || (t === 'eval' ? (x.cat === 'Size' ? 'g-box' : x.cat === 'Quality' ? 'g-chart' : x.cat === 'Cost' ? 'g-cloud' : 'g-gauge') : t === 'agent' ? 'g-term' : t === 'skill' ? 'g-book' : t === 'plugin' ? 'g-blocks' : (x.cat === 'Cloud' ? 'g-cloud' : x.cat === 'Code' ? 'g-git' : 'g-db'));
    const note = (m) => ({ cNotice: m });
    const mkConn = (cx) => ({ id: cx.id, name: cx.name + ' ' + cx.def, desc: cx.desc, cat: cx.cat, kind: cx.name, used: cx.used, added: true, gl: cx.gl, byk: cx.byk, ...(cx.cat === 'Cloud' ? { cloud: cx.id } : {}) });
    const mkEval = (e) => ({ ...e, cal: 'Today', runs: 9, ideas: K.GEN });
    const mkAgent = (g) => ({ id: g.id, name: g.name, desc: g.desc, provider: g.provider, model: g.model, via: g.via, cat: g.cat, used: 0, par: 2, gl: g.gl, byk: g.byk });
    const mkSkill = (k) => ({ ...k, yours: true });
    const addConn = (cx) => {
      if (isY('conn', cx.id)) return;
      const isCloud = cx.cat === 'Cloud';
      this.setState((st) => ({ connectors: [mkConn(cx), ...st.connectors], clouds: isCloud ? { ...st.clouds, [cx.id]: true } : st.clouds, ...note(cx.name + ' added to Yours.') }));
      this.log(isCloud ? 'cloud.connect' : 'source.add', isCloud ? cx.name : cx.name + ' · ' + cx.def);
    };
    const addEval = (e) => { if (isY('eval', e.id)) return; this.setState((st) => ({ evals: [...st.evals, mkEval(e)], ...note(e.name + ' added to Yours. It can now be chosen in a new experiment.') })); this.log('evaluation.add', e.name + ' · template'); };
    const addAgent = (g) => { if (isY('agent', g.id)) return; this.setState((st) => ({ agentList: [...st.agentList, mkAgent(g)], agents: { ...st.agents, [g.id]: true }, ...note(g.name + ' added to Yours and switched on.') })); this.log('agent.add', g.name + ' · ' + g.provider); };
    const addSkill = (k) => { if (isY('skill', k.id)) return; this.setState((st) => ({ skills: [...st.skills, mkSkill(k)], ...note(k.name + ' added to Yours. Agents can read it in new experiments.') })); this.log('skill.add', k.name + ' · v' + k.ver); };
    const addPlugin = (p) => {
      if (isY('plugin', p.id)) return;
      this.setState((st) => {
        const rec = { eval: [], skill: [], conn: [], agent: [] };
        let { evals, skills, connectors, agentList, agents, clouds } = st;
        p.items.eval.forEach((id) => { const e = K.EVCAT.find((q) => q.id === id); if (e && !evals.some((q) => q.id === id)) { evals = [...evals, mkEval(e)]; rec.eval.push(id); } });
        p.items.skill.forEach((id) => { const k = K.SKILLS.find((q) => q.id === id); if (k && !skills.some((q) => q.id === id)) { skills = [...skills, mkSkill(k)]; rec.skill.push(id); } });
        p.items.conn.forEach((id) => { const cx = K.CATALOG.find((q) => q.id === id); if (cx && !connectors.some((q) => q.id === id)) { connectors = [mkConn(cx), ...connectors]; rec.conn.push(id); if (cx.cat === 'Cloud') clouds = { ...clouds, [id]: true }; } });
        p.items.agent.forEach((id) => { const g = K.AGENTCAT.find((q) => q.id === id); if (g && !agentList.some((q) => q.id === id)) { agentList = [...agentList, mkAgent(g)]; agents = { ...agents, [id]: true }; rec.agent.push(id); } });
        const parts = KN.filter(([k]) => rec[k].length).map(([k, w]) => plural(rec[k].length, w));
        return { evals, skills, connectors, agentList, agents, clouds, plugins: [...st.plugins, p], pluginRec: { ...st.pluginRec, [p.id]: rec }, ...note(p.name + ' added. ' + (parts.length ? joinAnd(parts) + (parts.length === 1 && rec.eval.length + rec.skill.length + rec.conn.length + rec.agent.length === 1 ? ' was added to Yours.' : ' were added to Yours.') : 'Everything it contains was already in Yours.')) };
      });
      this.log('plugin.add', p.name + ' · ' + plCount(p));
    };
    const removePlugin = (p, withItems) => {
      this.setState((st) => {
        const rec = st.pluginRec[p.id] || {};
        const { [p.id]: _gone, ...rest } = st.pluginRec;
        const patch = { plugins: st.plugins.filter((q) => q.id !== p.id), pluginRec: rest, plRemove: null };
        let kept = 0, gone = 0;
        if (withItems) {
          const evG = (rec.eval || []).filter((id) => !st.opts.some((q) => q.ev === id));
          const skG = (rec.skill || []).filter((id) => !st.opts.some((q) => (q.skills || []).includes(id)));
          const cnG = rec.conn || [];
          const agG = (rec.agent || []).filter((id) => !st.opts.some((q) => q.agents.includes(id)));
          gone = evG.length + skG.length + cnG.length + agG.length;
          kept = (rec.eval || []).length + (rec.skill || []).length + cnG.length + (rec.agent || []).length - gone;
          patch.evals = st.evals.filter((e) => !evG.includes(e.id));
          patch.skills = st.skills.filter((k) => !skG.includes(k.id));
          patch.connectors = st.connectors.filter((c) => !cnG.includes(c.id));
          patch.agentList = st.agentList.filter((g) => !agG.includes(g.id));
          patch.clouds = { ...st.clouds, ...Object.fromEntries(st.connectors.filter((c) => cnG.includes(c.id) && c.cloud).map((c) => [c.cloud, false])) };
        }
        patch.cNotice = p.name + ' removed. ' + (withItems ? (gone ? plural(gone, 'item') + ' removed from Yours.' : 'Nothing else was removed.') + (kept ? ' ' + plural(kept, 'item') + ' stayed because experiments use ' + (kept === 1 ? 'it.' : 'them.') : '') : 'Its items stay in Yours.');
        return patch;
      });
      this.log('plugin.remove', p.name + (withItems ? ' · with its items' : ' · items kept'));
    };
    const openDet = (kind, id, extra) => () => this.setState({ screen: 'detail', menu: null, pal: false, detail: { kind, id }, cNotice: '', plRemove: null, skSec: 0, dsSel: 'traces', connSel: id, ...(extra || {}) });
    const adder = { eval: addEval, skill: addSkill, conn: addConn, agent: addAgent, plugin: addPlugin }[tab];
    const items = (list) => list.map((x, i) => ({ ...x, _i: i, _u: usedN(tab, x) }));

    // add menu and forms
    const openForm = (id) => () => this.setState({ form: id, ff: { a: '', b: '', c: '' }, fpick: {}, menu: null, cNotice: '' });
    const addItems = ({
      eval: [['Add evaluation from repository', openForm('evrepo')]],
      skill: [['Write a skill', openForm('skillw')], ['Import from repository', openForm('skillr')], ['Upload a folder', openForm('skillu')]],
      conn: [['Add custom connector', openForm('conn')], ['Connect a cloud', goCust({ cTab: 'conn', cView: 'discover', cCat: 'Cloud' })]],
      agent: [['Add a custom agent', openForm('agentc')]],
      plugin: [['Add from a Git URL', openForm('plgit')], ['Create a plugin from your items', openForm('plmake')]]
    })[tab].map(([label, go]) => ({ label, go }));
    const FORMS = {
      conn: ['Add custom connector', [['Name', 'a', 'Internal tools'], ['Server URL', 'b', 'https://mcp.acme.internal/tools', 1]], 'Add', ''],
      evrepo: ['Add evaluation from repository', [['Name', 'a', 'Cart load test'], ['Path', 'b', 'bench/cart-load.js', 1]], 'Add', ''],
      skillw: ['Write a skill', [['Name', 'a', 'Retry budgets'], ['Description', 'b', 'When and how to add a retry'], ['What agents should know', 'c', 'Write the guidance in plain sentences.', 2]], 'Add skill', 'Agents read this. The evaluator does not.'],
      skillr: ['Import skills from a repository', [['Folder in the repository', 'a', 'docs/agent-notes', 1], ['Name', 'b', 'Agent notes']], 'Import', 'Reads the Markdown files at the current commit of your repository.'],
      skillu: ['Upload a skill folder', [['Folder name', 'a', 'payments-playbook', 1], ['Name', 'b', 'Payments playbook']], 'Upload', 'The folder needs a SKILL.md at its top level.'],
      plgit: ['Add plugin from a Git URL', [['Git URL', 'a', 'https://github.com/acme/ratchet-kit.git', 1]], 'Add plugin', 'Its evaluations and skills are added to Yours.'],
      plmake: ['Create a plugin from your items', [['Name', 'a', 'Checkout kit'], ['Description', 'b', 'What this bundle is for']], 'Create plugin', 'Pick the items it should contain.'],
      agentc: ['Add a custom agent', [['Name', 'a', 'Internal agent'], ['Model', 'b', 'gateway/model-name', 1]], 'Add agent', '']
    };
    const frm = s.form ? FORMS[s.form] : null;
    const setFF = (k) => (e) => this.setState((st) => ({ ff: { ...st.ff, [k]: e.target.value } }));
    const submitForm = () => {
      const f = s.form, a = (s.ff.a || '').trim(), b = (s.ff.b || '').trim(), c = (s.ff.c || '').trim();
      const id = Date.now();
      const done = (tb, m, extra) => this.setState({ form: null, ff: { a: '', b: '', c: '' }, fpick: {}, cTab: tb, cView: 'yours', ...note(m), ...(extra || {}) });
      const slug = (x) => x.split(/[\/\\]/).filter(Boolean).pop() || x;
      const title = (x) => this.cap1(slug(x).replace(/\.git$/, '').replace(/[-_]+/g, ' '));
      const sk = (name, desc, sec) => ({ id: 'sk' + id, name, desc, cat: 'Conventions', byk: 'T', used: 0, ver: '1.0', ev: ['*'], sec, gl: 'g-book', yours: true });
      if (f === 'conn') { if (!a) return; this.setState((st) => ({ connectors: [{ id: 'x' + id, name: a, desc: b ? 'MCP server · ' + b : 'Custom MCP server', cat: 'Data', kind: 'MCP server', used: 0, added: true, gl: 'g-code' }, ...st.connectors] })); done('conn', a + ' added to Yours.'); this.log('source.add', 'MCP server · ' + a); }
      else if (f === 'evrepo') { if (!a) return; this.setState((st) => ({ evals: [...st.evals, { id: 'ev' + id, name: a, cat: 'Quality', desc: b ? 'Defined in ' + b : 'Defined in the repository', metric: 'Score from ' + (b || 'the repository'), fmt: ['', '', 2], dir: 'higher', base: 1, n: 2, cmd: b || 'bench/run.sh', cal: 'Not yet', runs: 0, used: 0, path: 'src', ho: 'ho_checkout', kw: a.toLowerCase(), ideas: K.GEN }] })); done('eval', a + ' added to Yours. It can now be chosen in a new experiment.'); this.log('evaluation.add', a + ' · ' + (b || 'repository')); }
      else if (f === 'skillw') { if (!a) return; this.setState((st) => ({ skills: [...st.skills, sk(a, b || 'Written by your team', [['SKILL.md', 'Overview', c || 'Describe what agents should know before they touch this part of the code.']])] })); done('skill', a + ' added to Yours. Agents can read it in new experiments.'); this.log('skill.add', a + ' · written'); }
      else if (f === 'skillr') { if (!a) return; const nm = b || title(a); this.setState((st) => ({ skills: [...st.skills, sk(nm, 'Imported from ' + a, [['SKILL.md', 'Overview', 'Imported from ' + a + ' at main@9f3c1e7. Agents read the Markdown files in this folder.'], ['SKILL.md', 'Files', 'One file per topic. Re-import to pick up newer commits.']])] })); done('skill', nm + ' imported from ' + a + '.'); this.log('skill.add', nm + ' · import'); }
      else if (f === 'skillu') { if (!a) return; const nm = b || title(a); this.setState((st) => ({ skills: [...st.skills, sk(nm, 'Uploaded from the folder ' + slug(a), [['SKILL.md', 'Overview', 'Uploaded from the folder ' + slug(a) + '. Agents read SKILL.md first, then the files it links to.']])] })); done('skill', nm + ' uploaded.'); this.log('skill.add', nm + ' · upload'); }
      else if (f === 'plgit') {
        if (!a) return;
        const nm = title(a), eid = 'ev' + id, sid = 'sk' + id;
        const ev = { id: eid, name: nm + ' check', cat: 'Quality', desc: 'Defined in the plugin repository', metric: 'Score from ' + nm, fmt: ['', '', 2], dir: 'higher', base: 1, n: 2, cmd: 'plugin/run.sh', cal: 'Not yet', runs: 0, used: 0, path: 'src', ho: 'ho_checkout', kw: nm.toLowerCase(), ideas: K.GEN };
        const p = { id: 'pl' + id, name: nm, desc: 'From ' + a, cat: 'Quality', byk: 'T', used: 0, gl: 'g-blocks', items: { eval: [eid], skill: [sid], conn: [], agent: [] } };
        this.setState((st) => ({ evals: [...st.evals, ev], skills: [...st.skills, { ...sk(nm + ' notes', 'Shipped with the ' + nm + ' plugin', [['SKILL.md', 'Overview', 'Guidance shipped in the plugin repository. Agents read it with the plugin evaluation.']]), id: sid }], plugins: [...st.plugins, p], pluginRec: { ...st.pluginRec, [p.id]: { eval: [eid], skill: [sid], conn: [], agent: [] } } }));
        done('plugin', nm + ' added. 1 evaluation and 1 skill were added to Yours.'); this.log('plugin.add', nm + ' · git');
      } else if (f === 'plmake') {
        if (!a) return;
        const pk = Object.keys(s.fpick).filter((k) => s.fpick[k]);
        const p = { id: 'pl' + id, name: a, desc: b || 'Bundle made from your own items', cat: 'Quality', byk: 'T', used: 0, gl: 'g-blocks', items: { eval: pk.filter((k) => k.startsWith('eval:')).map((k) => k.slice(5)), skill: pk.filter((k) => k.startsWith('skill:')).map((k) => k.slice(6)), conn: [], agent: [] } };
        this.setState((st) => ({ plugins: [...st.plugins, p] })); done('plugin', a + ' created with ' + (plCount(p) || 'no items') + '.'); this.log('plugin.add', a + ' · created');
      } else if (f === 'agentc') { if (!a) return; this.setState((st) => ({ agentList: [...st.agentList, { id: 'ag' + id, name: a, desc: 'Custom agent', provider: 'Bring your own model', model: b || 'configured per run', via: 'Acme gateway', cat: 'Open source', used: 0, par: 2, gl: 'g-bot' }], agents: { ...st.agents, ['ag' + id]: true } })); done('agent', a + ' added to Yours and switched on.'); this.log('agent.add', a + ' · custom'); }
    };

    // Yours rows
    const kindRows = {
      eval: () => s.evals.map((e) => ({ ...e, desc: e.metric, meta: 'Noise ±' + e.n + '% · calibrated ' + e.cal.toLowerCase(), status: 'Ready', stColor: 'var(--good)' })),
      skill: () => s.skills.map((k) => ({ ...k, meta: byTxt(k) + ' · v' + k.ver, status: skOn(k.id) ? 'On' : 'Off', stColor: skOn(k.id) ? 'var(--good)' : 'var(--ink3)' })),
      conn: () => s.connectors.map((c) => ({ ...c, meta: c.cat, status: s.reconnect[c.id] ? 'Reconnect' : 'Connected', stColor: s.reconnect[c.id] ? 'var(--warn)' : 'var(--good)' })),
      agent: () => s.agentList.map((g) => ({ ...g, desc: g.provider + ' · ' + g.model, meta: g.cat, status: s.agents[g.id] ? 'On' : 'Off', stColor: s.agents[g.id] ? 'var(--good)' : 'var(--ink3)' })),
      plugin: () => s.plugins.map((p) => ({ ...p, desc: plCount(p) || 'No items yet', meta: byTxt(p), status: 'Added', stColor: 'var(--good)' }))
    };
    const yours = items(kindRows[tab]()).filter(matchC).sort(sortC).map((x) => ({ ...x, gl: glFor(x, tab), open: openDet(tab, x.id) }));
    const yoursAll = poolOf[tab].length;

    // Discover
    const filteredC = !!cq || s.cCat !== 'All' || s.cAuth !== 'All' || s.cAdded !== 'All';
    const catItems = items(catOf[tab]);
    const featId = K.FEAT[tab];
    const featured = !filteredC && featId ? catItems.find((x) => x.id === featId) : null;
    const rest = catItems.filter((x) => !featured || x.id !== featured.id).filter(matchC).sort(sortC);
    const card = (x) => {
      const added = inY(x);
      return { gl: glFor(x, tab), mg: tab === 'conn' ? x.mg || '' : '', hasMg: tab === 'conn' && !!x.mg, hasGl: !(tab === 'conn' && x.mg), name: x.name, desc: x.desc, verified: tab === 'conn' && x.byk === 'R', added, notAdded: !added, add: () => adder(x), addLabel: 'Add ' + x.name, canOpen: tab === 'skill' || tab === 'plugin', plain: !(tab === 'skill' || tab === 'plugin'),
        meta: tab === 'plugin' ? plCount(x) + ' · ' + byTxt(x) : tab === 'skill' ? byTxt(x) + ' · used in ' + plural(usedN('skill', x), 'experiment') : x.by, open: openDet(tab, x.id) };
    };
    let secs;
    if (tab === 'conn') {
      const lim = s.showAll.conn ? rest.length : 8;
      secs = [{ title: filteredC ? 'Results' : K.MORE.conn, badge: String(rest.length), hasBadge: true, cards: rest.slice(0, lim).map(card), toggle: rest.length > 8 }];
    } else if (filteredC) secs = [{ title: 'Results', hasBadge: true, badge: String(rest.length), cards: rest.map(card), toggle: false }];
    else {
      const more = rest.slice(6), lim = s.showAll[tab] ? more.length : 4;
      secs = [{ title: 'For you', hasBadge: false, badge: '', cards: rest.slice(0, 6).map(card), toggle: false }, { title: K.MORE[tab], hasBadge: false, badge: '', cards: more.slice(0, lim).map(card), toggle: more.length > 4 }];
    }
    secs = secs.filter((x) => x.cards.length > 0);
    const bnTint = { eval: 'var(--accent)', skill: 'var(--good)', plugin: 'var(--infra)' }[tab] || 'var(--accent)';
    const bn = featured ? { ...card(featured), desc: K.FEATTXT[tab], tint: bnTint } : { gl: 'g-box', name: '', desc: '', meta: '', addLabel: 'Add', added: false, notAdded: true, tint: bnTint };
    const slide = K.SLIDES[((s.cSlide % 4) + 4) % 4];
    const showCar = tab === 'conn' && !filteredC && !yoursView;
    const carTiles = slide.ids.map((id, i) => { const c = K.CATALOG.find((q) => q.id === id); return { mg: c.mg, name: c.name, off: ['-8px', '10px', '-14px', '6px'][i] }; });
    const addSlide = (n) => () => this.setState((st) => ({ cSlide: (((st.cSlide + n) % 4) + 4) % 4 }));
    const catList = K.CATS[tab];
    const filterGroups = [{ title: 'Category', items: catList.map((c) => ({ label: c, on: c === s.cCat, pick: () => this.setState({ cCat: c, menu: null }) })) }, { title: 'Author', items: ['All', 'Ratchet', 'Community', 'Your team'].map((c) => ({ label: c, on: c === s.cAuth, pick: () => this.setState({ cAuth: c, menu: null }) })) }, ...(yoursView ? [] : [{ title: 'Status', items: ['All', 'Added', 'Not added'].map((c) => ({ label: c, on: c === s.cAdded, pick: () => this.setState({ cAdded: c, menu: null }) })) }])];
    const filtersOn = [s.cCat, s.cAuth, s.cAdded].filter((x) => x !== 'All').length;


    // detail pages
    const evUsers = (id) => s.opts.filter((x) => x.ev === id);
    const conn = detConn || s.connectors[0];
    const connDs = s.datasets.filter((x) => x.conn === conn.id);
    const dsSel = connDs.find((x) => x.id === s.dsSel) || connDs[0] || null;
    const setMark = (x, m) => {
      if (x.marking === m) return;
      this.setState((st) => ({
        datasets: st.datasets.map((y) => (y.id === x.id ? { ...y, marking: m } : y)),
        grants: m === 'Holdout' ? { ...st.grants, [x.id]: { ...st.grants[x.id], agents: false } } : st.grants,
        newSel: m === 'Holdout' ? { ...st.newSel, [x.id]: false } : st.newSel
      }));
      this.log('marking.set', x.name + ' · ' + m);
    };
    const flip = (x, who) => {
      const on = s.grants[x.id][who];
      this.setState((st) => ({ grants: { ...st.grants, [x.id]: { ...st.grants[x.id], [who]: !on } } }));
      this.log('grant.change', x.name + ' · ' + (who === 'agents' ? 'Agents' : 'Evaluator') + (on ? ' access removed' : ' access added'));
    };
    const usedBy = (x) => {
      if (x.marking === 'Holdout') { const u = s.opts.filter((q) => this.evOf(s, q.ev).ho === x.id).map((q) => q.name); return u.length ? u.join(', ') + ' · restricted to evaluator' : 'Restricted to evaluator'; }
      const u = s.opts.filter((q) => q.ds.includes(x.id)).map((q) => q.name);
      return u.length ? u.join(', ') : 'Not used yet';
    };
    const versOf = (x) => {
      if (x.vers) return x.vers;
      const m = /v(\d+)/.exec(x.version);
      const n = m ? Number(m[1]) : 1;
      const h = [...x.id].reduce((q, c, i) => q + c.charCodeAt(0) * (i + 1), 0) % 3;
      const dates = [['Yesterday', '2 weeks ago', 'Last month'], ['Today', '5 days ago', '3 weeks ago'], ['2 days ago', '9 days ago', 'Oct 14']][h];
      const m2 = /^([\d,.]+)([MKk]?)(.*)$/.exec(x.sz);
      const sz = (f) => { if (!m2) return x.sz; const raw = Number(m2[1].replace(/,/g, '')); const v = raw * f; const dec = m2[1].includes('.') ? 1 : 0; const out = dec ? v.toFixed(1) : Math.max(1, Math.round(v)).toLocaleString('en-US'); return out + m2[2] + m2[3]; };
      return [[x.version, dates[0], x.sz], ['v' + Math.max(1, n - 1), dates[1], sz(0.93)], ['v' + Math.max(1, n - 2), dates[2], sz(0.86)]];
    };
    const ck = conn.cloud || 'aws';
    const ci = Math.max(0, K.CLOUDS.findIndex((c) => c[0] === ck));
    const runMach = s.machines.filter((m) => m.cloud === K.CLOUDS[ci][1] && !m.gone);
    const goneN = s.machines.filter((m) => m.cloud === K.CLOUDS[ci][1] && m.gone).length;
    const aTrials = detAgent ? s.opts.flatMap((x) => { const ax = evAll(x); return ax.cur.filter((t) => t.a === detAgent.id && t.mes).map((t) => ({ opt: x.name, tried: t.text, result: this.fv(ax.ev, t.v) + ' (' + this.pt(t.p) + ')', out: K.OUT[t.out][0], color: K.OUT[t.out][1], open: openOpt(x) })); }) : [];
    const eExp = detEval ? evUsers(detEval.id).map((x) => ({ name: x.name, project: x.project, ...stOf(x), open: openOpt(x) })) : [];
    const aExp = detAgent ? s.opts.filter((x) => x.agents.includes(detAgent.id)).map((x) => ({ name: x.name, project: x.project, ...stOf(x), open: openOpt(x) })) : [];
    const detOn = detAgent ? !!s.agents[detAgent.id] : false;
    const skSecs = detSkill ? detSkill.sec.map(([f, h, b], i) => ({ f, h, b, i })) : [];
    const skSel = skSecs[Math.min(s.skSec, Math.max(0, skSecs.length - 1))] || null;
    const skTree = detSkill ? [...new Set(detSkill.sec.map((q) => q[0]))].map((f) => ({ file: f, items: skSecs.filter((q) => q.f === f).map((q) => ({ h: q.h, cls: skSel && q.i === skSel.i ? 'srow2 on' : 'srow2', pick: () => this.setState({ skSec: q.i }) })) })) : [];
    const skYours = detSkill ? isY('skill', detSkill.id) : false;
    const skUse = detSkill ? skUsers(detSkill.id) : [];
    const skExp = skUse.map((x) => ({ name: x.name, project: x.project, ...stOf(x), open: openOpt(x) }));
    const skAgents = [...new Set(skUse.flatMap((x) => x.agents))].map((id) => this.agName(s, id)).join(', ') || 'None yet';
    const skOnNow = detSkill ? skOn(detSkill.id) : false;
    const setSkOn = (on) => detSkill && this.setState((st) => ({ skillsOff: { ...st.skillsOff, [st.lastProject + ':' + detSkill.id]: !on } }));
    const plYours = detPlugin ? isY('plugin', detPlugin.id) : false;
    const plRec = detPlugin ? s.pluginRec[detPlugin.id] || {} : {};
    const plHasRec = KN.some(([k]) => (plRec[k] || []).length > 0);
    const plSecs = detPlugin ? [['eval', 'Evaluations'], ['skill', 'Skills'], ['conn', 'Connectors'], ['agent', 'Recommended agents']].filter(([k]) => detPlugin.items[k].length).map(([k, title]) => ({ title, rows: detPlugin.items[k].map((id) => { const it = resolve(k, id) || { name: id, desc: '' }; const added = isY(k, id); const canOpen = added || k === 'skill'; return { name: it.name, desc: it.desc || '', gl: glFor(it, k), mg: k === 'conn' ? it.mg || '' : '', hasMg: k === 'conn' && !!it.mg, hasGl: !(k === 'conn' && it.mg), added, stTxt: added ? 'Added' : 'Not added', stColor: added ? 'var(--good)' : 'var(--ink3)', canOpen, plain: !canOpen, open: openDet(k, id) }; }) })) : [];
    const plNew = detPlugin ? KN.reduce((n, [k]) => n + detPlugin.items[k].filter((id) => !isY(k, id)).length, 0) : 0;

    const people = [
      { id: 'priya', name: 'Priya Raman', email: 'priya@acme.com', source: 'okta:platform-leads' },
      { id: 'marco', name: 'Marco Bianchi', email: 'marco@acme.com', source: 'okta:perf-engineering' },
      { id: 'jun', name: 'Jun Park', email: 'jun@acme.com', source: 'invited by Priya' },
      { id: 'ci', name: 'ci-bot', email: 'Service principal · token ends 7f2a', source: 'created by Jun' }
    ];
    const actLabels = { 'optimization.create': 'Experiment created', 'run.start': 'Run started', 'run.stop': 'Run stopped', 'pull_request.open': 'Pull request opened', 'note.send': 'Note added', 'source.add': 'Connector added', 'source.remove': 'Connector removed', 'marking.set': 'Marking changed', 'grant.change': 'Permission changed', 'cloud.connect': 'Cloud connector added', 'agent.enable': 'Agent enabled', 'agent.disable': 'Agent disabled', 'agent.add': 'Agent added', 'evaluation.add': 'Evaluation added', 'evaluation.calibrate': 'Evaluation recalibrated', 'skill.add': 'Skill added', 'skill.remove': 'Skill removed', 'plugin.add': 'Plugin added', 'plugin.remove': 'Plugin removed', 'source.reconnect': 'Connector reconnected', 'budget.change': 'Budget changed', 'role.change': 'Role changed', 'signin.change': 'Single sign-on changed', 'token.create': 'Access token created', 'token.revoke': 'Access token revoked' };
    const actColor = (x) => {
      const p = x.split('.')[0];
      if (['optimization', 'run', 'pull_request', 'evaluation'].includes(p)) return 'var(--accent)';
      if (['source', 'marking', 'grant', 'note', 'skill', 'plugin'].includes(p)) return 'var(--good)';
      if (['cloud', 'agent', 'budget'].includes(p)) return 'var(--infra)';
      return 'var(--ink3)';
    };
    const orgSpent = totalSpend;
    const cphase = o.status === 'Draft' ? 'draft' : live ? 'live' : done || stopped ? 'end' : 'draft';
    const themeSeg = seg([['system', 'System'], ['light', 'Light'], ['dark', 'Dark']], s.theme || 'system', (v) => this.setState({ theme: v === 'system' ? null : v, menu: null }));

    // ---------- palette
    const pq = (s.palQ || '').trim().toLowerCase();
    const toks = pq.split(/\s+/).filter(Boolean);
    const PK = { action: ['Actions', null], experiment: ['Experiments', 'experiments'], trial: ['Trials', 'experiments'], note: ['Notes', 'experiments'], project: ['Projects', 'projects'], dataset: ['Datasets', 'datasets'], eval: ['Evaluations', 'customize'], skill: ['Skills', 'customize'], plugin: ['Plugins', 'customize'], agent: ['Agents', 'customize'], conn: ['Connectors', 'datasets'], person: ['People', 'people'], setting: ['Settings', 'people'], audit: ['Audit log', 'people'], run: ['Runs', 'runs'] };
    const PORD = Object.keys(PK);
    const PSC = { experiments: ['experiment', 'trial', 'note'], projects: ['project'], datasets: ['dataset', 'conn'], customize: ['eval', 'skill', 'plugin', 'agent', 'conn'], runs: ['run', 'trial'], people: ['person', 'setting', 'audit'] };
    const palScopes = [['all', 'All'], ['experiments', 'Experiments'], ['projects', 'Projects'], ['datasets', 'Datasets'], ['customize', 'Customize'], ['runs', 'Runs'], ['people', 'People']];
    const palGo = (patch) => () => this.setState({ pal: false, palAct: false, palQ: '', menu: null, confirmStop: false, renaming: false, ...patch });
    const splitAt = (text, key) => { const i = text.toLowerCase().indexOf(key); return i < 0 ? [text, '', ''] : [text.slice(0, i), text.slice(i, i + key.length), text.slice(i + key.length)]; };
    const mkItem = (kind, title, ctx, fields, go, extra) => {
      const tl = title.toLowerCase();
      if (toks.length && !toks.every((t) => (tl + ' ' + fields.join(' ').toLowerCase() + ' ' + ctx.toLowerCase()).includes(t))) return null;
      const needle = toks.length ? (tl.includes(pq) ? pq : toks.find((t) => tl.includes(t)) || '') : '';
      const [a, b, c] = needle ? splitAt(title, needle) : [title, '', ''];
      let ca = ctx, cb = '', cc = '';
      if (toks.length && !needle) {
        const f = fields.find((x) => x.toLowerCase().includes(pq)) || fields.find((x) => toks.some((t) => x.toLowerCase().includes(t)));
        if (f) {
          const key = f.toLowerCase().includes(pq) ? pq : toks.find((t) => f.toLowerCase().includes(t));
          const ix = f.toLowerCase().indexOf(key), st = Math.max(0, ix - 28);
          [ca, cb, cc] = splitAt((st > 0 ? '…' : '') + f.slice(st, st + 100), key);
        } else { const key = toks.find((t) => ctx.toLowerCase().includes(t)); if (key) [ca, cb, cc] = splitAt(ctx, key); }
      }
      return { kind, title, a, b, c, ca, cb, cc, go, score: needle ? (tl.startsWith(needle) ? 0 : 1) : 2, ic: extra.ic || 'i-search', color: extra.color || 'var(--ink2)', hint: extra.hint || '', ...extra };
    };
    const HINT = { Draft: 'Draft', Running: 'Running', Win: 'Win', 'Pending pull request': 'PR open', Merged: 'Merged', 'No win': 'No win', Stopped: 'Stopped' };
    const optGo = (x) => palGo({ screen: 'thread', selId: x.id, lastProject: x.project, projSel: x.project, hl: null });
    const openTrial = (x, key) => () => { this._scrollTo = 'tr-' + key.replace(':', '-'); palGo({ screen: 'thread', selId: x.id, lastProject: x.project, projSel: x.project, hl: key, ex: { ...s.ex, ['tr:' + key]: true } })(); };
    const settingsGo = (tb) => palGo({ screen: 'settings', settingsTab: tb });
    const custFind = (tb, name) => palGo({ screen: 'customize', ...custReset, cTab: tb, cView: 'discover', cQuery: name });
    const palItems = [];
    const P = (it) => { if (it) palItems.push(it); };
    const expAct = (x, ax) => ({ expId: x.id, pinned: x.pinned, again: this.isDone(x) || x.status === 'Stopped' ? () => { this.setState((st) => ({ opts: st.opts.map((q) => (q.id === x.id ? this.nextRun(q, ax, this.isWin(x) ? 'w' : x.status === 'Stopped' ? 's' : 'n') : q)), screen: 'thread', selId: x.id, pal: false, palAct: false, palQ: '', hl: null })); } : null });
    if (s.pal) {
      s.opts.forEach((x) => {
        const ax = evAll(x), t = st0(x);
        P(mkItem('experiment', x.name, x.project + ' · ' + t[0] + ' · ' + (x.act || 'Not started'), [x.req, ax.ev.metric, ax.ev.name], optGo(x), { ic: t[1], color: t[2], hint: HINT[x.status] || '', ...expAct(x, ax) }));
        ax.seq.forEach((tr) => P(mkItem('trial', tr.text, x.name + ' · run ' + tr.run + ' · ' + tr.agent + ' · ' + K.OUT[tr.out][0].toLowerCase(), [tr.agent, x.name], tr.key ? openTrial(x, tr.key) : optGo(x), { ic: 'g-code', hint: tr.mes ? this.pt(tr.p) : 'Measuring' })));
        for (let n = 1; n <= x.runs; n++) P(mkItem('run', x.name + ' · run ' + n, x.project + ' · ' + (n === x.runs ? t[0] : ({ w: 'A change was accepted', s: 'Stopped before a result', n: 'No change beat the baseline' })[x.pr[n - 1]] || 'Finished'), ['run history'], palGo({ screen: 'history', histTab: 'trials', histProj: x.project, histExp: x.name }), { ic: 'g-hist', hint: n === x.runs ? 'Latest' : '' }));
      });
      s.notes.forEach((n) => {
        const x = s.opts.find((q) => q.name === n.opt);
        const inThread = x && n.run === x.runs;
        P(mkItem('note', n.text, n.author + ' · ' + n.opt + ' · ' + n.time, [n.author, n.opt], inThread ? () => { this._scrollTo = 'note-' + n.id; palGo({ screen: 'thread', selId: x.id, lastProject: x.project, projSel: x.project, hlNote: n.id })(); setTimeout(() => this.setState((st) => (st.hlNote === n.id ? { hlNote: null } : null)), 2600); } : settingsGo('notes'), { ic: 'g-note', hint: n.time }));
      });
      K.PROJECTS.forEach(([n, role, blurb]) => P(mkItem('project', n, s.opts.filter((q) => q.project === n).length + ' experiments · ' + blurb, [blurb], palGo({ screen: 'project', projSel: n }), { ic: 'i-folder', hint: role })));
      s.datasets.forEach((x) => { const c = s.connectors.find((q) => q.id === x.conn); P(mkItem('dataset', x.name, (c ? c.name : '') + ' · ' + x.version + ' · ' + x.sz, [x.version, c ? c.name : ''], openDet('conn', x.conn, { dsSel: x.id }), { ic: 'g-db', hint: x.marking === 'Holdout' ? 'Holdout' : '' })); });
      const PL2 = { eval: ['eval', (x) => x.metric, 'g-gauge'], skill: ['skill', (x) => 'Skill · v' + (x.ver || '1.0') + ' · ' + byTxt(x), 'g-book'], plugin: ['plugin', (x) => plCount(x) || 'Plugin', 'g-blocks'], agent: ['agent', (x) => (x.provider || '') + ' · ' + (x.model || ''), 'g-term'], conn: ['conn', (x) => 'Connector · ' + x.cat, 'g-git'] };
      Object.keys(PL2).forEach((k) => {
        const [kind, ctxf, ic] = PL2[k];
        const have = poolOf[k];
        have.forEach((x) => P(mkItem(kind, x.name, ctxf(x), [x.desc || '', x.cat || ''], openDet(k, x.id), { ic: x.gl || ic, hint: k === 'skill' ? (skOn(x.id) ? 'On' : 'Off') : k === 'agent' ? (s.agents[x.id] ? 'On' : 'Off') : k === 'conn' && s.reconnect[x.id] ? 'Reconnect' : 'Added', color: k === 'conn' && s.reconnect[x.id] ? 'var(--warn)' : 'var(--ink2)' })));
        catOf[k].filter((x) => !have.some((q) => q.id === x.id)).forEach((x) => P(mkItem(kind, x.name, ctxf(x), [x.desc || '', x.cat || ''], k === 'skill' || k === 'plugin' ? openDet(k, x.id) : custFind(k, x.name), { ic: x.gl || ic, hint: 'Not added' })));
      });
      [['priya', 'Priya Raman', 'priya@acme.com', 'okta:platform-leads'], ['marco', 'Marco Bianchi', 'marco@acme.com', 'okta:perf-engineering'], ['jun', 'Jun Park', 'jun@acme.com', 'an invitation'], ['ci', 'ci-bot', 'Service principal', 'a service principal']].forEach(([id, nm, em, src]) => P(mkItem('person', nm, em + ' · added through ' + src, [em], settingsGo('members'), { ic: 'g-user', hint: s.roles[id] })));
      const SETKW = { general: 'organization theme default budget dark light appearance', notes: 'guidance notes agents read', budgets: 'budget spend monthly cap limit cost', members: 'users groups roles invite owner editor viewer people access', signin: 'okta sso sign-in session login identity', tokens: 'access token api key cli mcp ci service', audit: 'audit log history changes record' };
      settingsTabs.forEach(([id, label]) => P(mkItem('setting', label, 'Settings', [SETKW[id]], settingsGo(id), { ic: 'i-sliders' })));
      s.audit.forEach((e) => P(mkItem('audit', (actLabels[e.action] || e.action) + ' · ' + e.resource, e.who + ' · ' + e.time, [e.action, e.who], settingsGo('audit'), { ic: 'g-file', hint: e.time })));
      const dk = theme === 'dark';
      [['New experiment', 'Start a draft from a goal', 'i-plus', palGo({ screen: 'new', editId: null, d: { ...s.d, text: '', name: null, ev: null, path: null, dir: null, project: null, skills: null }, needText: false }), 'create start draft goal'],
        ['Add a connector', 'Customize · Connectors', 'i-plus', palGo({ screen: 'customize', ...custReset, cTab: 'conn', cView: 'discover' }), 'connect source warehouse repository'],
        ['Add a skill', 'Customize · Skills', 'g-book', palGo({ screen: 'customize', ...custReset, cTab: 'skill', cView: 'discover' }), 'skills know-how guidance'],
        ['Open Dashboard', 'Value, spend and win rate', 'i-chart', palGo({ screen: 'value' }), 'dashboard value return wins'],
        ['Invite a user', 'Settings · Users and groups', 'g-user', settingsGo('members'), 'people member add team'],
        [dk ? 'Switch to light theme' : 'Switch to dark theme', 'Appearance', dk ? 'i-sun' : 'i-moon', () => this.setState({ pal: false, palAct: false, palQ: '', theme: dk ? 'light' : 'dark' }), 'theme appearance dark light mode']
      ].forEach(([t, c, ic, go, kw]) => P(mkItem('action', t, c, [kw], go, { ic })));
    }
    const inScope = (it) => s.palScope === 'all' || PSC[s.palScope].includes(it.kind);
    const palGroups = [];
    const flat = [];
    if (s.pal) {
      if (!pq && s.palScope === 'all') {
        const need = [];
        s.opts.forEach((x) => {
          const it = palItems.find((q) => q.kind === 'experiment' && q.expId === x.id);
          if (x.status === 'Draft' && !x.approved) need.push({ ...it, ca: 'Draft awaiting approval · ' + x.project, hint: 'Approve', color: 'var(--accent)' });
          if ((this.isDone(x) || x.status === 'Stopped') && this.spend(x) >= x.cap) need.push({ ...it, ca: 'Reached its ' + money(x.cap) + ' budget · ' + x.project, hint: 'Budget', color: 'var(--warn)' });
          if (x.status === 'Win') need.push({ ...it, ca: 'Accepted win, pull request not opened · ' + x.project, hint: 'Open PR', color: 'var(--good)' });
        });
        s.connectors.filter((c) => s.reconnect[c.id]).forEach((c) => need.push({ ...palItems.find((q) => q.kind === 'conn' && q.title === c.name), ca: 'Needs reconnect · access token expired', hint: 'Reconnect', color: 'var(--warn)' }));
        const rec = [];
        const PG = { projects: ['Projects', 'i-folder'], value: ['Dashboard', 'i-chart'], history: ['Run history', 'g-hist'] };
        s.recent.forEach((k) => {
          const [t0, a1, a2] = k.split(':');
          const it = t0 === 'x' ? palItems.find((q) => q.kind === 'experiment' && q.expId === a1)
            : t0 === 'p' ? palItems.find((q) => q.kind === 'project' && q.title === a1)
            : t0 === 'd' ? palItems.find((q) => q.kind === a1 && q.title === (resolve(a1, a2) || {}).name)
            : t0 === 's' ? palItems.find((q) => q.kind === 'setting' && q.title === (settingsTabs.find((z) => z[0] === a1) || [])[1])
            : t0 === 'g' && PG[a1] ? { kind: 'page', title: PG[a1][0], a: PG[a1][0], b: '', c: '', ca: 'Page', cb: '', cc: '', ic: PG[a1][1], color: 'var(--ink2)', hint: '', go: palGo({ screen: a1 }) } : null;
          if (it && !rec.some((q) => q.title === it.title && q.kind === it.kind) && !need.some((q) => q.title === it.title && q.kind === it.kind)) rec.push(it);
        });
        [['Needs attention', need.slice(0, 4)], ['Recents', rec.slice(0, 6)], ['Actions', palItems.filter((q) => q.kind === 'action')]].forEach(([title, items]) => { if (items.length) palGroups.push({ title, items }); });
      } else {
        const sel = palItems.filter(inScope).sort((x, y) => x.score - y.score);
        const kinds = s.palScope === 'all' ? PORD : s.palScope === 'runs' ? ['run', 'trial'] : PORD.filter((k) => PSC[s.palScope].includes(k));
        const lim = s.palScope === 'all' ? 5 : 12;
        kinds.forEach((k) => {
          const list = sel.filter((q) => q.kind === k);
          if (!list.length) return;
          palGroups.push({ title: PK[k][0], items: list.slice(0, lim), n: list.length, more: s.palScope === 'all' && list.length > lim && PK[k][1] ? PK[k][1] : null });
        });
      }
      palGroups.forEach((g) => g.items.forEach((it) => flat.push(it)));
    }
    const palHi = Math.max(0, Math.min(s.palHi, flat.length - 1));
    const curIt = flat[palHi] || null;
    const palActs = curIt ? [['Open', curIt.go], ...(curIt.kind === 'experiment' ? [[curIt.pinned ? 'Unpin' : 'Pin', () => this.setState((st) => ({ opts: st.opts.map((q) => (q.id === curIt.expId ? { ...q, pinned: !q.pinned } : q)), palAct: false }))]] : []), ...(curIt.again ? [['Run again', curIt.again]] : [])] : [];
    this._pal = { items: flat, acts: palActs, hi: palHi, scopes: palScopes.map((x) => x[0]) };
    let ix = -1;
    const palRows = palGroups.map((g) => ({
      title: g.title, more: !!g.more, moreLabel: g.more ? 'Show all ' + g.n : '', showMore: () => this.setState({ palScope: g.more, palHi: 0, palAct: false }),
      rows: g.items.map((it) => {
        ix++; const my = ix; const on = my === palHi;
        return { dom: 'pr-' + my, on, cls: on ? 'pr on' : 'pr', hover: () => { if (this.state.palHi !== my) this.setState({ palHi: my }); }, open: () => it.go(), ic: it.ic, color: it.color, a: it.a, b: it.b, c: it.c, hasMatch: !!it.b, ca: it.ca, cb: it.cb, cc: it.cc, hasCm: !!it.cb, hint: it.hint, hasHint: !!it.hint, actions: (e) => { if (e && e.stopPropagation) e.stopPropagation(); this.setState((st) => ({ palHi: my, palAct: st.palHi === my ? !st.palAct : true, palActHi: 0 })); }, aria: it.title };
      })
    }));

