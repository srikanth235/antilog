# A screen

Mockup first, then the web app (target). There is no client app yet.

## 1. The mockup

Start in `design/web/`. Edit the parts in `design/web/src/` (a changed value goes in `style.css`) and run `bash design/web/src/assemble.sh`. See [`design/README.md`](../../../design/README.md). Behaviour and words: [design](../design/SKILL.md).

## 2. The web app (target)

(target) A screen is built for the web client. It is React and TypeScript over the same API and the same behaviour and words. It does not exist yet. See [stack/typescript.md](../stack/typescript.md). If the screen needs new data, do [api-endpoint.md](api-endpoint.md) first.

The mockup is `design/web/`. Build to its shared patterns: one detail-page layout for every kind under Customize, one popover, one palette for global search. The shape of the app is in [design behaviour](../design/behaviour.md); the words are in [ui-terms](../design/ui-terms.md).
