# Notifications (target)

A notification tells a person that something needs them or that work they asked about is done. It is never progress chatter.

## What notifies, and whom

| Event | Who gets it | Words (title · line) |
| --- | --- | --- |
| A win: a trial was Kept and a win is ready | whoever started the experiment, and anyone who asked to be notified | Win · the experiment title, then the measured change |
| Waiting for you: a card needs approval, or the agent asked a question | whoever started the experiment, and the editors and owners of its project | Waiting for you · the experiment title, then what to decide |
| A regression: a watched experiment measured worse than delivered by more than the noise | whoever started the experiment, and owners | Regression · the experiment title, then the change |
| A budget was spent | whoever started the experiment, and owners | Budget spent · the experiment title |
| Patience reached: an experiment paused on a Keep going? card | the editors and owners of its project | Waiting for you · the experiment title, then Keep going? |
| A session needs another agent (agent unavailable) | the editors and owners of its project | Waiting for you · the experiment title, then Continue on another agent? |
| An experiment's work finished (Done or Failed) | whoever started the experiment, only if they asked ("Notify me when this finishes") | Done or Failed · the experiment title |

A trial that breaks a guardrail is only recorded and notifies no one. The words come from the one catalogue and the glossary ([../design/wording.md](../design/wording.md)); a notification never says something the app would not.

## Where it shows

- **macOS:** native notifications from the Mac app (target), set in its Settings window.
- **Windows and Linux, and any browser:** web push from the web app, installed or in a tab, set per device in Settings, This device. Asked for only from a user action (after starting an experiment), never on load.
- **The icon badge** counts the experiments that are Waiting for you and clears as they are seen.
- **In the app**, the sidebar and the experiment page already show the same states; a notification only points there.

## The monthly report email

The report is a page, built (`GET /report`, `antilog-core/src/value.rs`); the email is target. One email a month with the organization's or a project's report, to the members an owner chose ("Email this monthly"). The report holds spent, Delivered, Pending, At risk, Return, where the spend went, the agent scoreboard for the month, the top wins with receipt links and experiments stopped with no win. It carries numbers, titles and links, never diffs, tool output or data, and only to members who may read that report. It is the one email Antilog sends; a person can leave the list. Choosing the recipients and exporting the report are audited actions. A self-hosted server that has no mail endpoint configured sends none, and the report is still on its page.

No other email, chat or webhook notifications for now. Adding one is a product call.

## Rules

- **Ids only leave the server.** A push payload is an experiment or session id and an event kind. The client fetches the rest and builds the text from its own words. Nothing about data, diffs or logs is in a notification.
- **One event, one notification per person.** Collapse repeats with the same tag (the experiment id), and a newer state replaces an older one.
- **Per person, per device.** Turning notifications off on one device does not touch another. The server keeps each device's push subscription with the person, and deletes it when the browser revokes it or the person signs out there.
- **Notifications are reads.** They change no state, so they are not audited (the monthly report's recipient list is); turning them on or off on a device is not audited either. A notification is sent after the event is committed, from the same live event the app receives ([../change/live-event.md](../change/live-event.md)).
- **A self-hosted server that cannot reach the browsers' push services sends no web push.** Everything else works; say so in Settings, This device.
