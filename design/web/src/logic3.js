
    return {
      appCls: 'app ' + theme, isDark: theme === 'dark', isLight: theme === 'light',
      setScroll: this.setScroll,
      hasMenu: !!menu, closeMenu: () => this.setState({ menu: null }),
      isNew: is('new'), isThread: is('thread'), isProjects: is('projects'), isProject: is('project'), isValue: is('value'), isCustomize: is('customize'), isDetail: is('detail'), isHistory: is('history'),
      crumbs: crumbRows, topNew: is('project') && roleOf(s.projSel) !== 'Viewer',
      topNewGo: () => this.setState({ screen: 'new', menu: null, editId: null, d: { ...s.d, text: '', name: null, ev: null, path: null, dir: null, project: s.projSel, skills: null }, needText: false }),

      // sidebar
      openPal: () => this.openPal(),
      pal: s.pal, palQ: s.palQ, palRows, palCount: flat.length, palScopes: palScopes.map(([id, label]) => ({ label, cls: s.palScope === id ? 'ptab on' : 'ptab', sel: s.palScope === id, pick: () => { this.setState({ palScope: id, palHi: 0, palAct: false }); this.palIn && this.palIn.focus(); } })),
      palEmpty: !!s.pal && flat.length === 0, palEmptyTxt: 'No matches for “' + (s.palQ || '').trim() + '”.',
      startAbout: () => { const q = (s.palQ || '').trim(); palGo({ screen: 'new', editId: null, d: { ...s.d, text: q, name: null, ev: null, path: null, dir: null, project: null, skills: null }, needText: false })(); },
      onPalQ: (e) => this.setState({ palQ: e.target.value, palHi: 0, palAct: false }), closePal: () => this.closePal(), setPalIn: this.setPalIn,
      palActive: curIt ? 'pr-' + palHi : '', palActOpen: s.pal && s.palAct && palActs.length > 0, palActTitle: curIt ? curIt.title : '',
      palActRows: palActs.map(([label, go], i) => ({ label, cls: i === s.palActHi ? 'mi on' : 'mi', hover: () => this.setState({ palActHi: i }), pick: () => go() })),
      palBox: () => this.palIn && this.palIn.focus(),
      palShow: s.palScope === 'all' && !pq ? 'Search experiments, projects, datasets and more' : 'Search',
      navProjectsCls: nav('projects'), goProjects: goTo('projects'),
      navValueCls: nav('value'), goValue: goTo('value'),
      navCustomizeCls: nav('customize'), goCustomize: goCust({}),
      navMoreCls: is('history') ? 'nav on' : menu === 'more' ? 'nav open' : 'nav', moreOpen: menu === 'more', toggleMore: tmenu('more'),
      goHistory: goTo('history', { histTab: 'trials', histProj: 'All', histExp: 'All' }), goSettingsUser: () => this.openSettings(null, '#btn-user'), goSettingsMore: () => this.openSettings(null, '#btn-more'),
      newCls: is('new') ? 'btn primary full on' : 'btn primary full',
      goNew: () => this.setState({ screen: 'new', menu: null, editId: null, d: { ...s.d, text: '', name: null, ev: null, path: null, dir: null, project: null, skills: null }, needText: false, confirmStop: false, renaming: false }),
      hasPinned: pinned.length > 0, pinned, groups,
      userOpen: menu === 'user', toggleUser: tmenu('user'), hostName: 'ratchet.acme.internal',
      themeLabel: theme === 'dark' ? 'Switch to light' : 'Switch to dark',
      toggleTheme: () => this.setState({ theme: theme === 'dark' ? 'light' : 'dark', menu: null }),

      // composer
      dText: d.text, onText: (e) => this.setState((st) => ({ d: { ...st.d, text: e.target.value }, needText: false })),
      needText: s.needText && !(d.text || '').trim(),
      vProject: dProj, vSkills: selSk.length ? plural(selSk.length, 'skill') : 'No skills', mSkills: mopen('p:skills'), tSkills: tmenu('p:skills'), cSkills: mopen('p:skills') ? 'pill on' : 'pill',
      skillOpts: onSk.map((k) => ({ name: k.name, meta: 'v' + k.ver, on: selSkIds.includes(k.id), fill: selSkIds.includes(k.id) ? 'var(--good)' : 'transparent', toggle: () => setD({ skills: selSkIds.includes(k.id) ? selSkIds.filter((i) => i !== k.id) : [...selSkIds, k.id] }) })), noSkillOn: onSk.length === 0, goSkills: goCust({ cTab: 'skill', cView: 'yours' }),
      pluginSugg: K.PSUGG.filter(([pid, evid]) => !isY('eval', evid) && !isY('plugin', pid)).slice(0, 2).map(([pid, , label]) => ({ label, open: openDet('plugin', pid) })),
      vEval: dEv.name, vData: selIds.length + (selIds.length === 1 ? ' dataset' : ' datasets'), vEnv: cloudLabel(d.env), vAgents: plural(d.agents, 'agent'), vBudget: money(d.cap || 0),
      mProj: mopen('p:proj'), mEv: mopen('p:ev'), mData: mopen('p:data'), mEnv: mopen('p:env'), mAgents: mopen('p:agents'), mBudget: mopen('p:budget'), mGuard: mopen('p:guard'),
      tProj: tmenu('p:proj'), tEv: tmenu('p:ev'), tData: tmenu('p:data'), tEnv: tmenu('p:env'), tAgents: tmenu('p:agents'), tBudget: tmenu('p:budget'), tGuard: tmenu('p:guard'),
      cProj: mopen('p:proj') ? 'pill on' : 'pill', cEv: mopen('p:ev') ? 'pill on' : 'pill', cData: mopen('p:data') ? 'pill on' : 'pill', cEnv: mopen('p:env') ? 'pill on' : 'pill', cAgents: mopen('p:agents') ? 'pill on' : 'pill', cBudget: mopen('p:budget') ? 'pill on' : 'pill', cGuard: mopen('p:guard') ? 'pill quiet on' : 'pill quiet',
      projOpts: K.PROJECTS.map(([n, role]) => ({ label: n, role: role === 'Viewer' ? 'Viewer, cannot start experiments' : role, on: n === dProj, dis: role === 'Viewer', pick: () => { setD({ project: n }); this.setState({ menu: null }); } })),
      evOpts: s.evals.map((e) => ({ name: e.name, metric: e.metric + ' · ' + e.dir + ' is better', on: e.id === dEv.id, pick: () => { setD({ ev: e.id, dir: null, path: null }); this.setState({ menu: null }); } })),
      goEvals: goCust({ cTab: 'eval', cView: 'yours' }),
      dataGroups: s.connectors.map((c) => ({ name: c.name, items: s.datasets.filter((x) => x.conn === c.id && x.marking !== 'Holdout').map((x) => ({ name: x.name, meta: x.version, on: !!s.newSel[x.id], fill: s.newSel[x.id] ? 'var(--good)' : 'transparent', toggle: () => this.setState((st) => ({ newSel: { ...st.newSel, [x.id]: !st.newSel[x.id] } })) })) })).filter((g) => g.items.length > 0),
      holdName: dHold ? dHold.name : 'No dataset is marked Holdout',
      goConnectors: goCust({ cTab: 'conn', cView: 'yours' }),
      envSeg: K.CLOUDS.map(([k, , label]) => ({ label, note: s.clouds[k] ? '' : 'Connect', cls: !s.clouds[k] ? 'chip dis' : (d.env === k ? 'chip on' : 'chip'), pick: () => (s.clouds[k] ? (setD({ env: k }), this.setState({ menu: null })) : goCust({ cTab: 'conn', cView: 'discover', cCat: 'Cloud' })()) })),
      dAgents: d.agents, agentNames: enabled.length ? enabled.map((i) => this.agName(s, i)).join(', ') : 'No agent is on',
      agentsDown: () => setD({ agents: Math.max(1, d.agents - 1) }), agentsUp: () => setD({ agents: Math.min(8, d.agents + 1) }),
      dCap: d.cap, onCap: (e) => setD({ cap: digits(e) }),
      dirSeg: seg([['lower', 'Lower is better'], ['higher', 'Higher is better']], dDir, (v) => setD({ dir: v })),
      limitChips: K.LIMITS.map(([k, l]) => ({ label: l, cls: d.limits[k] ? 'chip on' : 'chip', pick: () => setD({ limits: { ...d.limits, [k]: !d.limits[k] } }) })),
      dPath: d.path !== null && d.path !== undefined ? d.path : '', pathHint: dEv.path, onPath: (e) => setD({ path: e.target.value }),
      dNameVal: d.name || '', nameHint: dName, onName: (e) => setD({ name: e.target.value }),
      draftCls: canDraft ? 'btn primary' : 'btn primary off', noDraft: !canDraft,
      suggestions: K.SUGG.map(([text]) => ({ label: text, pick: () => this.setState((st) => ({ d: { ...st.d, text, name: null, ev: null, dir: null, path: null }, needText: false, menu: null })) })),
      goDraft: () => {
        if (!(d.text || '').trim()) { this.setState({ needText: true, menu: null }); return; }
        if (roleOf(dProj) === 'Viewer') return;
        const ids = enabled.length ? enabled : ['claude'];
        const ags = Array.from({ length: d.agents }, (_, i) => ids[i % ids.length]);
        const fields = { skills: selSkIds, project: dProj, req: d.text.trim(), ev: dEv.id, dir: d.dir, lim: { ...d.limits }, path: d.path, env: d.env, cloud: cloudLabel(d.env), cap: Number(d.cap) || 200, agents: ags, ds: [...selIds] };
        const reset = { text: '', name: null, ev: null, path: null, dir: null, project: null, skills: null };
        if (s.editId) {
          const old = s.opts.find((x) => x.id === s.editId);
          const nn = old && old.runs === 1 ? dName : old.name;
          this.setState((st) => ({
            opts: st.opts.map((x) => (x.id === s.editId ? { ...x, ...fields, name: nn } : x)),
            notes: nn !== old.name ? st.notes.map((n) => (n.opt === old.name ? { ...n, opt: nn } : n)) : st.notes,
            screen: 'thread', selId: s.editId, editId: null, menu: null, lastProject: dProj, projSel: dProj, d: { ...st.d, ...reset }
          }));
          return;
        }
        const id = 'n' + Date.now();
        this.setState((st) => ({
          opts: [{ id, name: dName, status: 'Draft', run: 1, runs: 1, step: 0, fixed: null, pinned: false, approved: false, hist: [], pr: [], ps: 0, val: 0, mo: 5, act: 'Just now', T: [], ...fields }, ...st.opts],
          screen: 'thread', selId: id, editId: null, menu: null, lastProject: dProj, projSel: dProj, collapsed: { ...st.collapsed, [dProj]: false }, d: { ...st.d, ...reset }, hl: null
        }));
      },

      // thread header
      tTitle: o.name, renaming: s.renaming, notRenaming: !s.renaming, showStopBtn: live && !s.confirmStop, noBest: !hasBest, renameVal: s.renameVal,
      onRename: (e) => this.setState({ renameVal: e.target.value }),
      onRenameKey: (e) => { if (e.key === 'Enter') this.saveRename(); if (e.key === 'Escape') this.setState({ renaming: false }); },
      saveRename: () => this.saveRename(), cancelRename: () => this.setState({ renaming: false }),
      startRename: () => this.setState({ renaming: true, renameVal: o.name, menu: null }),
      ...stOf(o),
      mName: ev.metric, mTip: 'Primary metric: ' + ev.metric, mBase: this.fv(ev, ev.base), hasBest: hasBest, mBest: hasBest ? this.fv(ev, a.best.v) : '', mDelta: hasBest ? this.pt(a.best.p) : '',
      mDeltaColor: hasBest && ['accepted', 'pend'].includes(a.best.out) ? 'var(--good)' : 'var(--ink2)',
      runNo: o.runs, trialN: String(trialCount), trialSub: a.past.length && o.step >= 4 ? a.cur.length + ' in this run' : '',
      spendTxt: money(oSpend), capTxt: money(o.cap), spendPct: Math.min(100, Math.round((oSpend / o.cap) * 100)) + '%',
      elapsed: o.step ? this.fmtMin(K.MIN[Math.min(o.step, 8)]) : 'Not started',
      confirmStop: s.confirmStop, askStop: () => this.setState({ confirmStop: true, menu: null }), cancelStop: () => this.setState({ confirmStop: false }),
      doStop: () => {
        upd(o.id, { status: 'Stopped', act: 'Just now' });
        this.setState((st) => ({ confirmStop: false, machines: st.machines.map((m) => (m.opt === o.name && !m.gone ? { ...m, gone: true } : m)) }));
        this.log('run.stop', o.name + ' · step ' + o.step);
      },
      pinTxt: o.pinned ? 'Unpin' : 'Pin', pinTip: (o.pinned ? 'Unpin ' : 'Pin ') + o.name, isPinned: o.pinned, togglePin: () => upd(o.id, { pinned: !o.pinned }),
      tMoreOpen: mopen('t:more'), toggleTMore: tmenu('t:more'),
      goHistThread: goTo('history', { histTab: 'trials', histProj: o.project, histExp: o.name }),

      // chart
      hasChart, noChart: !hasChart, chartTitle: ev.metric + ' per trial', chartHint: ev.dir === 'lower' ? 'Lower is better' : 'Higher is better',
      bandTop: pct(Y(ev.base + band)), bandH: pct(Y(ev.base - band) - Y(ev.base + band)), baseTop: pct(Y(ev.base)), baseLbl: 'Baseline ' + this.fv(ev, ev.base),
      ticks, seps, segs, dots, noChartTxt: 'No trials yet. The chart fills in as agents propose changes.',

      // conversation
      reqText: o.req, reqChips, pastRuns, sayDraft, declLines: dc.lines, acceptLine: 'Acceptance criteria: a change must beat twice the noise (' + (2 * ev.n).toFixed(1) + '%) and pass the holdout.',
      detOpen, detSummary: 'Draft approved by Priya Raman · ' + dc.summary, toggleDet: toggleEx(exKey('det')), detLabel: s.ex[exKey('det')] ? 'Hide details' : 'Show details',
      isDraft: !approvedAt, isApproved: approvedAt, detClosed: approvedAt && !detOpen, planOpen: !!s.ex[exKey('plan')], togglePlan: toggleEx(exKey('plan')), planLabel: s.ex[exKey('plan')] ? 'Hide plan' : 'Show plan', plan,
      editDraft: () => this.setState({ screen: 'new', menu: null, editId: o.id, d: { text: o.req, ev: o.ev, dir: o.dir, limits: { ...o.lim }, path: o.path, name: o.name, env: o.env, agents: o.agents.length, cap: String(o.cap), project: o.project, skills: [...(o.skills || [])] }, newSel: Object.fromEntries(o.ds.map((i) => [i, true])), needText: false }),
      approve: () => {
        if (o.approved) return;
        this.setState((st) => ({ opts: st.opts.map((x) => (x.id === o.id ? { ...x, status: 'Running', step: 1, approved: true, T: this.genT(x, st), act: 'Just now' } : x)) }));
        if (o.runs === 1) this.log('optimization.create', o.name);
        this.log('run.start', o.name + ' · run #' + o.runs);
      },
      showProg, progSay: stopped ? 'The run was stopped. Running trials were discarded and nothing new will start.' : done ? 'Run ' + o.runs + ' is complete. All hosts are terminated and clean-up is verified.' : 'Approved. Run ' + o.runs + ' is under way.',
      nodes, stepLine: stopped ? 'Stopped at step ' + o.step + ' of 8.' : o.step >= 8 ? 'Step 8 of 8 · ' + this.fmtMin(K.MIN[8]) + ' in total.' : 'Step ' + o.step + ' of 8 · ' + stepText.charAt(0).toLowerCase() + stepText.slice(1),
      stepsOpen: !!s.ex[exKey('steps')], toggleSteps: toggleEx(exKey('steps')), stepsLabel: s.ex[exKey('steps')] ? 'Hide steps' : 'Show steps', prog,
      canAdvance: live && o.step < 8,
      advance: () => {
        if (!live || o.step >= 8) return;
        const n = o.step + 1;
        const slug = o.name.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '').slice(0, 14);
        this.setState((st) => {
          let machines = st.machines;
          let opts = st.opts.map((x) => (x.id === o.id ? { ...x, step: n } : x));
          if (n === 2 && !machines.some((m) => m.opt === o.name && m.role === 'runner' && !m.gone)) machines = [{ id: 'mr' + Date.now(), name: 'runner-' + slug + '-' + o.runs, role: 'runner', opt: o.name, cloud: o.cloud, age: '0 m', gone: false }, ...machines];
          if (n === 5 && !machines.some((m) => m.opt === o.name && m.role === 'evaluator' && !m.gone)) machines = [{ id: 'me' + Date.now(), name: 'eval-' + slug + '-' + o.runs, role: 'evaluator', opt: o.name, cloud: o.cloud, age: '0 m', gone: false }, ...machines];
          if (n === 8) {
            machines = machines.map((m) => (m.opt === o.name && !m.gone ? { ...m, gone: true, age: this.fmtMin(K.MIN[8]) } : m));
            const w = this.an({ ...opts.find((x) => x.id === o.id), step: 8 }, st).win;
            opts = opts.map((x) => (x.id === o.id ? { ...x, status: w ? 'Win' : 'No win', fixed: null } : x));
          }
          return { opts, machines };
        });
      },
      showTrials: o.step >= 4 && trials.length > 0, trialsTitle: 'Trials · run ' + o.runs, trials,
      threadNotes,
      showResult: done || stopped,
      resSay: stopped ? 'This run was stopped before a result. Here is what had been measured.' : resSay,
      hasWin: !!winT && done, noWin: !winT && (done || stopped),
      resHead: winPct, resSum, resChange: winT ? winT.text : '', resWho: winT ? winT.agent + ' · ' + winT.files + (winT.files === 1 ? ' file' : ' files') + ', +' + winT.add + ' −' + winT.del : '',
      lineage: [{ title: 'Experiment', color: 'var(--accent)', rows: optCol }, { title: 'Data', color: 'var(--good)', rows: ctxCol }, { title: 'Compute', color: 'var(--infra)', rows: infCol }],
      triedLine: a.mc ? 'Tried: ' + a.cur.filter((x) => x.mes).map((x) => x.text + ' (' + K.OUT[x.out][0].toLowerCase() + ')').join('; ') + '.' : '', noWinText: stopped ? 'Stopped at step ' + o.step + '. ' + (a.mc ? 'These trials were measured before the stop.' : 'No trial had been measured yet.') : 'No trial met the acceptance criteria' + (nHold ? '. ' + (nHold === 1 ? 'One change' : nHold + ' changes') + ' beat the baseline but did not hold up on data the agents never saw.'        : '. Every trial stayed within noise or got worse.') + (hitCap && !stopped ? ' The run stopped when the budget was reached.' : ''),
      noWinTitle: stopped ? 'Run stopped' : 'No win', noWinHint: stopped ? 'Run again to start a new draft. A note below tells agents where to look first.' : 'Add a note about where to look, or raise the budget, then run again.',
      canPr: o.status === 'Win', prLine: o.status === 'Pending pull request' ? 'Pull request #' + (298 + s.opts.indexOf(o)) + ' is open for review.' : o.status === 'Merged' ? 'Merged by Priya Raman.' : '',
      openPr: () => { if (o.status !== 'Win') return; upd(o.id, { status: 'Pending pull request', act: 'Just now' }); this.log('pull_request.open', o.name + ' · PR #' + (319 + s.opts.length)); },
      runAgain: () => this.setState((st) => ({ opts: st.opts.map((x) => (x.id === o.id ? this.nextRun(x, a, this.isWin(o) ? 'w' : stopped ? 's' : 'n') : x)), hl: null })),
      placeholder: ph, threadDraft: s.threadDraft, noText: !(s.threadDraft || '').trim(),
      onThreadDraft: (e) => this.setState({ threadDraft: e.target.value }),
      onThreadKey: (e) => { if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); sendThread(); } },
      sendNote: sendThread, sendLabel: cphase === 'end' ? 'Start run' : 'Send',

      // projects
      projectCards: projList.map((p) => {
        const w = p.list.filter((x) => this.isWin(x)).length;
        return { name: p.n, role: p.role, blurb: p.blurb, count: String(p.list.length), wins: String(w), spend: money(p.list.reduce((t, x) => t + this.spend(x), 0)), pick: goTo('project', { projSel: p.n }) };
      }),
      pName: pj.n, pBlurb: pj.blurb,
      pStats: [['Experiments', String(pj.list.length)], ['Wins', String(pj.list.filter((x) => this.isWin(x)).length)], ['Spend this month', money(pj.list.reduce((t, x) => t + this.spend(x), 0))], ['Your role', pj.role]].map(([k, v]) => ({ k, v })),
      pRows: pj.list.map((x) => ({ name: x.name, ...stOf(x), metric: metricTxt(x), runs: String(x.runs), spend: money(this.spend(x)), act: x.act || 'Not started', open: openOpt(x), pinned: x.pinned })),
      pEmpty: pj.list.length === 0,

      // dashboard
      dTiles: [
        { k: 'Value per year', v: kfmt(totalVal), sub: 'from ' + winN + (winN === 1 ? ' accepted win' : ' accepted wins'), color: 'var(--ink)' },
        { k: 'Spent', v: money(totalSpend), sub: 'agents ' + money(Math.round(totalSpend * 0.49)) + ' · evaluation hosts ' + money(Math.round(totalSpend * 0.41)), color: 'var(--ink)' },
        { k: 'Return', v: (totalSpend ? Math.round(totalVal / totalSpend) : 0) + '×', sub: 'yearly value over spend', color: 'var(--accent)' },
        { k: 'Win rate', v: winN + ' of ' + finishedRuns, sub: 'finished runs ended in an accepted change', color: 'var(--ink)' }
      ],
      dgrid, dsegs, ddots, dmonths: K.MONTHS.map((m, i) => ({ t: m, left: xpos(i) })), dTotal: kfmt(totalVal),
      dWins: sortedWins.slice().reverse().map((w) => { const wi = winInfo(w); return { name: w.name, project: w.project, delta: this.pt(wi.p), metric: wi.ax.ev.metric, val: kfmt(wi.val), ...stOf(w), open: openOpt(w) }; }),

      // history
      histIsTrials: s.histTab === 'trials', histIsInst: s.histTab === 'inst',
      histTabs: [['trials', 'Trials'], ['inst', 'Instances']].map(([id, label]) => ({ label, cls: s.histTab === id ? 'tab on' : 'tab', pick: () => this.setState({ histTab: id }) })),
      histProj: s.histProj, histExp: s.histExp,
      onHistProj: (e) => this.setState({ histProj: e.target.value, histExp: 'All' }), onHistExp: (e) => this.setState({ histExp: e.target.value }),
      projOptions: ['All', ...K.PROJECTS.map((p) => p[0])].map((v) => ({ v, l: v === 'All' ? 'All projects' : v })),
      expOptions: hExpOptions.map((v) => ({ v, l: v === 'All' ? 'All experiments' : v })),
      historyRows: histAll.filter(hFilter), histEmpty: histAll.filter(hFilter).length === 0,
      machRows, machEmpty: machRows.length === 0,

      // customize
      cTabs: K.TABS.map(([id, label]) => ({ label, sel: tab === id, cls: tab === id ? 'ptab on' : 'ptab', pick: () => this.setState({ cTab: id, ...custReset, menu: null }) })),
      cViews: seg([['yours', 'Yours'], ['discover', 'Discover']], s.cView, (v) => this.setState({ cView: v, menu: null, cNotice: '', justAdded: {}, cAdded: 'All' })),
      isYours: yoursView, isDiscover: !yoursView,
      cQuery: s.cQuery, onCQuery: (e) => this.setState({ cQuery: e.target.value }), cPlaceholder: 'Search ' + K.NOUN[tab],
      filterCls: filtersOn || menu === 'filter' ? 'icon-btn on' : 'icon-btn', filterOpen: menu === 'filter', toggleFilter: tmenu('filter'),
      filterTip: filtersOn ? 'Filters on: ' + filtersOn : 'Filter', filterGroups,
      sortOpen: menu === 'sort', toggleSort: tmenu('sort'), sortTip: 'Sort: ' + ({ az: 'A to Z', used: 'Most used', new: 'Newest' })[s.cSort],
      sortOpts: [['az', 'A to Z'], ['used', 'Most used'], ['new', 'Newest']].map(([v, l]) => ({ label: l, on: s.cSort === v, pick: () => this.setState({ cSort: v, menu: null }) })),
      hasFilterTags: filtersOn > 0, filterTags: [['Category', s.cCat], ['Author', s.cAuth], ['Status', s.cAdded]].filter((x) => x[1] !== 'All').map((x) => ({ t: x[0] + ': ' + x[1] })),
      clearFilters: () => this.setState({ cCat: 'All', cAuth: 'All', cAdded: 'All' }),
      cNotice: s.cNotice, hasNotice: !!s.cNotice,
      addOpen: menu === 'add', toggleAdd: tmenu('add'), addItems: addItems.map((x) => ({ ...x, go: x.go })),
      hasForm: !!frm, formTitle: frm ? frm[0] : '', formSubmit: frm ? frm[2] : 'Add', formHelp: frm ? frm[3] : '', hasHelp: !!(frm && frm[3]),
      formFields: frm ? frm[1].map(([label, k, ph, kind]) => ({ label, value: s.ff[k], change: setFF(k), ph, isArea: kind === 2, isLine: kind !== 2, mono: kind === 1 ? 'in mono' : 'in' })) : [],
      formHasPicks: s.form === 'plmake',
      formPicks: [...s.skills.map((k) => ({ key: 'skill:' + k.id, name: k.name, meta: 'Skill' })), ...s.evals.map((e) => ({ key: 'eval:' + e.id, name: e.name, meta: 'Evaluation' }))].map((x) => ({ ...x, fill: s.fpick[x.key] ? 'var(--good)' : 'transparent', on: !!s.fpick[x.key], toggle: () => this.setState((st) => ({ fpick: { ...st.fpick, [x.key]: !st.fpick[x.key] } })) })),
      submitForm, closeForms: () => this.setState({ form: null }),
      yourRows: yours, yoursNone: yoursAll === 0, yoursNoMatch: yoursAll > 0 && yours.length === 0,
      yoursEmptyTxt: 'No ' + K.NOUN[tab] + ' in Yours yet.', yoursEmptyHint: ({ eval: 'Evaluations decide whether a change counts as a win.', skill: 'Skills are written know-how that agents read before they propose a change.', conn: 'Connectors let agents read your code and data inside your cloud.', agent: 'Agents propose the changes that evaluations then measure.', plugin: 'A plugin adds evaluations, skills and connectors for one domain in a single step.' })[tab],
      goDiscover: () => this.setState({ cView: 'discover', menu: null, cNotice: '' }),
      hasBanner: !!featured, bn, bnAdd: () => featured && !inY(featured) && adder(featured), bnOpen: openDet(tab, featId || ''), bnCanOpen: tab === 'skill' || tab === 'plugin', bnEyebrow: 'From Ratchet', illE: tab === 'eval', illS: tab === 'skill', illP: tab === 'plugin',
      showCar, carTitle: slide.t, carDesc: slide.d, carTint: slide.tint, carTiles, carPrev: addSlide(-1), carNext: addSlide(1), carExplore: () => this.setState({ cCat: slide.cat, menu: null }), carExploreLabel: 'Explore ' + slide.t.toLowerCase(),
      carDots: [0, 1, 2, 3].map((i) => ({ cls: i === ((s.cSlide % 4) + 4) % 4 ? 'dotp on' : 'dotp', label: 'Show featured slide ' + (i + 1), pick: () => this.setState({ cSlide: i }) })),
      secs, allLabel: s.showAll[tab] ? 'Show fewer' : 'Show all', toggleAll: () => this.setState((st) => ({ showAll: { ...st.showAll, [tab]: !st.showAll[tab] } })),
      discEmpty: rest.length === 0 && !featured ? 'Nothing matches your filters.' : '',

      // detail
      dIsEval: detKind === 'eval', dIsAgent: detKind === 'agent', dIsCData: detKind === 'conn' && conn.cat !== 'Cloud', dIsCCloud: detKind === 'conn' && conn.cat === 'Cloud',
      dBack: goCust({ cTab: detKind, cView: 'yours' }), dBackLabel: 'Back to ' + K.TABN[detKind].toLowerCase(),
      dNotice: s.cNotice, dHasNotice: !!s.cNotice,
      dGl: gl(detSkill ? 'g-book' : detPlugin ? 'g-blocks' : detEval ? glFor(detEval, 'eval') : detAgent ? glFor(detAgent, 'agent') : glFor(conn, 'conn')),
      dName: detName, dDesc: detSkill ? detSkill.desc : detPlugin ? detPlugin.desc : detEval ? detEval.desc : detAgent ? detAgent.desc : conn.desc,
      dStatus: detSkill ? (skYours ? (skOnNow ? 'On for ' + s.lastProject : 'Off for ' + s.lastProject) : 'Not added') : detPlugin ? (plYours ? 'Added' : 'Not added') : detEval ? 'Ready' : detAgent ? (detOn ? 'On' : 'Off') : s.reconnect[conn.id] ? 'Needs reconnect' : 'Connected',
      dStatusColor: detSkill ? (skYours && skOnNow ? 'var(--good)' : 'var(--ink3)') : detPlugin ? (plYours ? 'var(--good)' : 'var(--ink3)') : detAgent && !detOn ? 'var(--ink3)' : detKind === 'conn' && s.reconnect[conn.id] ? 'var(--warn)' : 'var(--good)',
      dMeta: detSkill ? 'Version ' + detSkill.ver + ' · ' + byTxt(detSkill) + ' · used in ' + plural(usedN('skill', detSkill), 'experiment') : detPlugin ? plCount(detPlugin) + ' · ' + byTxt(detPlugin) : '',
      dIsSkill: !!detSkill, dIsPlugin: !!detPlugin,
      skYours, skNotYours: !skYours, skOnCls: skOnNow ? 'seg on' : 'seg', skOffCls: skOnNow ? 'seg' : 'seg on', skSetOn: () => setSkOn(true), skSetOff: () => setSkOn(false), skProject: s.lastProject,
      skAdd: () => detSkill && addSkill(detSkill),
      skRemove: () => { if (!detSkill) return; this.setState((st) => ({ skills: st.skills.filter((k) => k.id !== detSkill.id), screen: 'customize', cTab: 'skill', cView: 'yours', ...custReset, cNotice: detSkill.name + ' removed from Yours.' })); this.log('skill.remove', detSkill.name); },
      skTree, hasSkSel: !!skSel, skH: skSel ? skSel.h : '', skF: skSel ? skSel.f : '', skBody: skSel ? skSel.b : '',
      dHasMeta: !!(detSkill || detPlugin),
      skKv: detSkill ? [['Version', detSkill.ver], ['Author', authOf(detSkill)], ['Files', plural(skTree.length, 'file') + ' · ' + plural(skSecs.length, 'section')], ['Read by agents', skAgents], ['Holdout data', 'Never included']].map(([k, v]) => ({ k, v })) : [],
      plYours, plNotYours: !plYours, plAdd: () => detPlugin && addPlugin(detPlugin), plSecs,
      plAskRemove: () => { if (!detPlugin) return; if (plHasRec) this.setState({ plRemove: detPlugin.id }); else removePlugin(detPlugin, false); },
      plRemoving: !!detPlugin && s.plRemove === detPlugin.id, plKeep: () => detPlugin && removePlugin(detPlugin, false), plDrop: () => detPlugin && removePlugin(detPlugin, true), plCancel: () => this.setState({ plRemove: null }),
      plAddNote: detPlugin && !plYours ? (plNew ? 'Adds ' + plNew + (plNew === 1 ? ' item' : ' items') + ' to Yours.' : 'Everything it contains is already in Yours.') : '',
      dReconnect: detKind === 'conn' && !!s.reconnect[conn.id],
      reconnect: () => { this.setState((st) => ({ reconnect: { ...st.reconnect, [conn.id]: false }, cNotice: conn.name + ' reconnected.' })); this.log('connector.reconnect', conn.name); },
      // eval
      eKv: detEval ? [['Primary metric', detEval.metric], ['Unit', detEval.fmt[1].trim() || detEval.fmt[0] || 'score'], ['Direction', this.cap1(detEval.dir) + ' is better'], ['Baseline', this.fv(detEval, detEval.base)], ['Command', detEval.cmd]].map(([k, v]) => ({ k, v, mono: k === 'Command' ? 'mono xs' : '' })) : [],
      eCal: detEval ? [['Last calibrated', detEval.cal], ['Noise', '±' + detEval.n + '% over ' + detEval.runs + ' baseline runs'], ['Acceptance', 'A win must beat twice the noise (' + (2 * detEval.n).toFixed(1) + '%)']].map(([k, v]) => ({ k, v })) : [],
      eHold: detEval ? ((s.datasets.find((x) => x.id === detEval.ho) || {}).name || 'None') : '',
      dHasExp: !!(detEval || detAgent || detSkill), dExp: detEval ? eExp : detSkill ? skExp : aExp, dExpEmpty: (detEval ? eExp : detSkill ? skExp : aExp).length === 0,
      recalibrate: () => { if (!detEval) return; this.setState((st) => ({ evals: st.evals.map((e) => (e.id === detEval.id ? { ...e, cal: 'Today' } : e)), cNotice: 'Recalibrated just now. Noise is ±' + detEval.n + '% over ' + detEval.runs + ' fresh baseline runs.' })); this.log('evaluation.calibrate', detEval.name + ' · noise ±' + detEval.n + '%'); },
      // agent
      aKv: detAgent ? [['Model provider', detAgent.provider], ['Reached through', detAgent.via]].map(([k, v]) => ({ k, v })) : [],
      aModel: detAgent ? detAgent.model : '', onAModel: (e) => detAgent && this.setState((st) => ({ agentList: st.agentList.map((g) => (g.id === detAgent.id ? { ...g, model: e.target.value } : g)) })),
      aPar: detAgent ? detAgent.par : 0, aParDown: () => detAgent && this.setState((st) => ({ agentList: st.agentList.map((g) => (g.id === detAgent.id ? { ...g, par: Math.max(1, g.par - 1) } : g)) })), aParUp: () => detAgent && this.setState((st) => ({ agentList: st.agentList.map((g) => (g.id === detAgent.id ? { ...g, par: Math.min(8, g.par + 1) } : g)) })),
      aOnCls: detOn ? 'seg on' : 'seg', aOffCls: detOn ? 'seg' : 'seg on', aSetOn: () => detAgent && setAgentOn(detAgent, true), aSetOff: () => detAgent && setAgentOn(detAgent, false),
      aTrials, aTrialsEmpty: aTrials.length === 0,
      // connector
      disconnect: () => {
        if (conn.id === 'rh') return;
        this.setState((st) => ({ connectors: st.connectors.filter((c) => c.id !== conn.id), clouds: conn.cloud ? { ...st.clouds, [conn.cloud]: false } : st.clouds, d: conn.cloud && st.d.env === conn.cloud ? { ...st.d, env: 'aws' } : st.d, screen: 'customize', cTab: 'conn', cView: 'yours', ...custReset }));
        this.log('connector.remove', conn.kind + ' · ' + conn.name);
      },
      canDisconnect: detKind === 'conn' && conn.id !== 'rh',
      dsRows: connDs.map((x) => {
        const g = s.grants[x.id] || { agents: true, evaluator: true };
        const hold = x.marking === 'Holdout';
        return {
          name: x.name, version: x.version, selCls: dsSel && x.id === dsSel.id ? 'sel' : '', pick: () => this.setState({ dsSel: x.id }), usedBy: usedBy(x),
          marks: seg([['None'], ['Holdout']], x.marking, (m) => setMark(x, m)), hold, notHold: !hold,
          aCls: g.agents ? 'tog on' : 'tog', aLabel: g.agents ? 'Can read' : 'No access', aOn: g.agents, aToggle: () => flip(x, 'agents'),
          eCls: g.evaluator ? 'tog on' : 'tog', eLabel: g.evaluator ? 'Can read' : 'No access', eOn: g.evaluator, eToggle: () => flip(x, 'evaluator')
        };
      }),
      dsEmpty: connDs.length === 0, hasDs: !!dsSel, dsName: dsSel ? dsSel.name : '', dsVersions: dsSel ? versOf(dsSel).map(([v, created, size]) => ({ v, created, size })) : [],
      markHelp: dsSel && dsSel.marking === 'Holdout' ? 'Marked Holdout. Restricted to evaluator. Agents cannot read it, and their access is removed.' : 'Agents can read it when permitted.',
      cloudKv: [['Account', ck === 'aws' ? 'acme-prod · us-east-1 · role ratchet' : 'acme-staging · role ratchet'], ['Environment', K.CLOUDS[ci][2]], ['Machine type', ['c7i.2xlarge', 'n2-standard-8', 'D8s v5'][ci]], ['Image', ['ratchet-bench-2026.11', 'ratchet-bench-gcp-2026.11', 'ratchet-bench-az-2026.11'][ci]]].map(([k, v]) => ({ k, v, mono: k === 'Machine type' || k === 'Image' ? 'mono xs' : '' })),
      runMach, runCount: runMach.length, runEmpty: runMach.length === 0, goneLink: goneN + (goneN === 1 ? ' terminated instance' : ' terminated instances') + ' in Run history',
      goInstances: goTo('history', { histTab: 'inst', histProj: 'All', histExp: 'All' }),

      // settings
      setOpen: s.setOpen, setQ: s.setQ, closeSettings: () => this.closeSettings(), setSdBox: this.setSdBox, setMainRef: this.setMainRef,
      onSetQ: (e) => this.setState({ setQ: e.target.value }), clearSetQ: () => this.setState({ setQ: '' }),
      onSetSel: (e) => this.setState({ settingsTab: e.target.value }), setTab: effTab,
      railGroups: SETGRP.map(([gid, gt]) => ({ title: gt, items: visSecs.filter((x) => x[3] === gid).map((x) => ({ id: x[0], label: x[1], ic: x[2], cur: x[0] === effTab, cls: x[0] === effTab ? 'sd-nav on' : 'sd-nav', ro: gid === 'org' && (x[0] === 'notes' ? noteRO : x[0] === 'audit' ? false : orgRO), go: () => this.setState({ settingsTab: x[0] }) })) })).filter((g) => g.items.length),
      noSetHits: visSecs.length === 0, sec: secOn, show: rowOn, grp: grpOn, myRole, orgRO, noteRO,
      noTokens: s.tokens.length === 0, noNotes: s.notes.length === 0, auditQ: s.auditQ, onAuditQ: (e) => this.setState({ auditQ: e.target.value }), noAudit: auditRows.length === 0,
      hasRoleNote: !!s.roleNote, roleNote: s.roleNote,
      defCap: s.defCap, onDefCap: (e) => { if (!isOwner) return; const v = digits(e); this.setState((st) => ({ defCap: v, d: { ...st.d, cap: v } })); },
      themeSeg, noteRows: s.notes, noteDraft: s.noteDraft, onNoteDraft: (e) => this.setState({ noteDraft: e.target.value }),
      onNoteKey: (e) => { if (e.key === 'Enter') this.addNoteGlobal(); }, addNote: () => this.addNoteGlobal(), noNote: noteRO || !(s.noteDraft || '').trim(),
      orgSpentText: money(orgSpent), monthlyText: money(s.monthly), orgPct: Math.min(100, Math.round((orgSpent / s.monthly) * 100)) + '%',
      capDraft: s.capDraft, onCapDraft: (e) => this.setState({ capDraft: digits(e) }),
      setMonthly: () => { const v = Number(s.capDraft); if (!v || !isOwner) return; this.setState({ monthly: v, capDraft: '' }); this.log('budget.change', 'Total monthly budget · ' + money(v)); },
      noCap: orgRO || !Number(s.capDraft),
      budgetRows: s.opts.map((x) => ({ name: x.name, pct: Math.min(100, Math.round((this.spend(x) / x.cap) * 100)) + '%', text: money(this.spend(x)) + ' of ' + money(x.cap), open: openOpt(x) })),
      members: people.map((p) => ({ ...p, init: p.name.charAt(0), role: s.roles[p.id], ro: orgRO, roles: ['Owner', 'Editor', 'Viewer'].map((r) => ({ v: r })), pick: (e) => setRole(p, e.target.value) })),
      sessionSeg: seg([['8 hours'], ['12 hours'], ['24 hours']], s.session, (v) => { if (isOwner && v !== s.session) { this.setState({ session: v }); this.log('signin.change', 'Session length · ' + v); } }),
      hasNewToken: s.newToken,
      createToken: () => { if (s.newToken) return; this.setState({ newToken: true, tokens: [{ id: 't' + Date.now(), name: 'New access token', tail: 'b4c0', kind: 'Personal', scope: 'Everything you can', used: 'Never', expires: 'in 90 days' }, ...s.tokens] }); this.log('token.create', 'New access token · ends b4c0'); },
      tokens: s.tokens.map((t) => ({ ...t, revoke: () => { this.setState({ tokens: s.tokens.filter((x) => x.id !== t.id), newToken: t.tail === 'b4c0' ? false : s.newToken }); this.log('token.revoke', t.name + ' · ends ' + t.tail); } })),
      audit: auditRows
    };
  }
  saveRename() {
    const s = this.state;
    const o = s.opts.find((x) => x.id === s.selId);
    const nn = (s.renameVal || '').trim();
    if (!nn || !o) { this.setState({ renaming: false }); return; }
    this.setState((st) => ({
      renaming: false,
      opts: st.opts.map((x) => (x.id === o.id ? { ...x, name: nn } : x)),
      notes: st.notes.map((n) => (n.opt === o.name ? { ...n, opt: nn } : n)),
      machines: st.machines.map((m) => (m.opt === o.name ? { ...m, opt: nn } : m))
    }));
  }
  addNoteGlobal() {
    const text = (this.state.noteDraft || '').trim();
    if (!text) return;
    this.setState((st) => ({ notes: [{ id: 'nn' + Date.now(), text, author: 'Priya Raman', opt: 'All experiments', time: this.now(), run: 0 }, ...st.notes], noteDraft: '' }));
    this.log('note.send', 'All experiments · note');
  }

  openPal() {
    this._prevFocus = document.activeElement;
    this.setState({ pal: true, palQ: '', palScope: 'all', palHi: 0, palAct: false, menu: null });
  }
  closePal() { this.setState({ pal: false, palAct: false, palQ: '' }); }
  openSettings(tab, fall, row) {
    const s = this.state;
    if (!s.setOpen) { this._setPrev = fall ? null : (s.pal ? this._prevFocus : document.activeElement); this._setFall = fall || null; }
    if (row) this._scrollTo = 'sr-' + row;
    this.setState((st) => ({ setOpen: true, settingsTab: tab || st.settingsTab, setQ: '', menu: null, pal: false, palAct: false, palQ: '', recent: tab ? ['s:' + tab, ...st.recent.filter((q) => q !== 's:' + tab)].slice(0, 10) : st.recent }));
  }
  closeSettings() { this.setState({ setOpen: false, setQ: '' }); }
  compose() {
    const s = this.state, q = (s.palQ || '').trim();
    this._compose = true;
    if (s.screen === 'thread') {
      this.setState({ pal: false, palAct: false, palQ: '', setOpen: false, threadDraft: q || s.threadDraft });
      setTimeout(() => { const ta = document.querySelector('.dock textarea'); if (ta) ta.focus(); }, 40);
    } else {
      this.setState({ pal: false, palAct: false, palQ: '', setOpen: false, screen: 'new', editId: null, menu: null, d: { ...s.d, text: q || s.d.text, name: null, ev: null, path: null, dir: null, project: null, skills: null }, needText: false });
      setTimeout(() => { const ta = document.querySelector('.composer textarea'); if (ta) ta.focus(); }, 40);
    }
  }
  onKey(e) {
    const s = this.state;
    const mod = e.metaKey || e.ctrlKey;
    if (mod && (e.key === 'k' || e.key === 'K')) { e.preventDefault(); if (s.pal) this.closePal(); else this.openPal(); return; }
    if (!s.pal) {
      if (s.setOpen && e.key === 'Tab' && this.sdBox) {
        const f = Array.from(this.sdBox.querySelectorAll('a[href],button:not([disabled]),input:not([disabled]),select:not([disabled]),[tabindex]:not([tabindex="-1"])')).filter((x) => x.offsetParent !== null);
        if (f.length) {
          const first = f[0], last = f[f.length - 1], cur = document.activeElement;
          if (e.shiftKey && (cur === first || cur === this.sdBox)) { e.preventDefault(); last.focus(); }
          else if (!e.shiftKey && cur === last) { e.preventDefault(); first.focus(); }
          else if (!this.sdBox.contains(cur)) { e.preventDefault(); first.focus(); }
        }
        return;
      }
      if (e.key === '/' && !mod && !e.altKey) {
        const t = e.target, tag = t && t.tagName;
        if (!(tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT' || (t && t.isContentEditable))) { e.preventDefault(); this.openPal(); }
        return;
      }
      if (e.key === 'Escape') {
        if (s.menu) this.setState({ menu: null });
        else if (s.setOpen) { e.preventDefault(); const t = e.target; if (t && t.tagName === 'INPUT' && s.setQ && t.getAttribute('aria-label') === 'Search settings') this.setState({ setQ: '' }); else this.closeSettings(); }
        else if (s.renaming) this.setState({ renaming: false });
      }
      return;
    }
    const P = this._pal || { items: [], acts: [], hi: 0, scopes: ['all'] };
    const k = e.key;
    if (k === 'Escape') { e.preventDefault(); if (s.palAct) this.setState({ palAct: false }); else this.closePal(); return; }
    if (k === 'Tab') { e.preventDefault(); if (e.shiftKey) { if (this.palIn) this.palIn.focus(); } else this.compose(); return; }
    if (s.palAct) {
      const n = P.acts.length;
      if (k === 'ArrowDown') { e.preventDefault(); this.setState({ palActHi: (s.palActHi + 1) % n }); }
      else if (k === 'ArrowUp') { e.preventDefault(); this.setState({ palActHi: (s.palActHi + n - 1) % n }); }
      else if (k === 'Enter') { e.preventDefault(); const a = P.acts[s.palActHi]; if (a) a[1](); }
      return;
    }
    if (k === 'ArrowDown' || k === 'ArrowUp') {
      e.preventDefault();
      const n = P.items.length;
      if (!n) return;
      this._hiKb = true;
      this.setState({ palHi: k === 'ArrowDown' ? (P.hi + 1) % n : (P.hi + n - 1) % n });
    } else if ((k === 'ArrowLeft' || k === 'ArrowRight') && (!s.palQ || e.altKey)) {
      e.preventDefault();
      const i = P.scopes.indexOf(s.palScope);
      this.setState({ palScope: P.scopes[(i + (k === 'ArrowRight' ? 1 : P.scopes.length - 1)) % P.scopes.length], palHi: 0 });
    } else if (k === 'Enter') {
      e.preventDefault();
      if (e.altKey) { if (P.items.length) this.setState({ palAct: true, palActHi: 0 }); } else if (P.items[P.hi]) P.items[P.hi].go();
    }
  }
}
