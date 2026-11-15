# Notifications (target)

Decided by the owner, 15 Nov; replaces "Notifications cut" (13 Nov). A notification tells a person that something needs them or that work they asked about is done. It is never progress chatter.

## What notifies, and whom

| Event | Who gets it | Words (title · line) |
| --- | --- | --- |
| A win is ready | whoever started the run, and anyone who asked to be notified | Win · the experiment name, then the measured change |
| Needs attention: measuring is blocked (the evaluation failed 3 times in a row), or a decision is waiting. A trial that breaks a guardrail is only discarded and notifies no one | whoever started the run, and editors and owners of the project | Needs attention · the experiment name, then what to decide |
| The run ended with no win, failed or stopped | whoever started the run, only if they asked ("Notify me when this run finishes") | No win, Failed or Stopped · the experiment name |
| A budget was hit | whoever started the run, and owners | Stopped at the budget · the experiment name |
| A runner is not connected | owners | Runner not connected · its name |

The words come from the one catalogue and the glossary; a notification never says something the app would not.

## Where it shows

- **macOS:** native notifications from the Mac app, set in its Settings window.
- **Windows and Linux, and any browser:** web push from the web app, installed or in a tab, set per device in Settings, This device. Asked for only from a user action (after starting a run), never on load.
- **The icon badge** counts the experiments that need attention and clears as they are seen.
- **In the app**, Home and the thread already show the same states; a notification only points there.

No email, chat or webhook notifications for now. Adding one is a product call.

## Rules

- **Ids only leave the server.** A push payload is an experiment id and an event kind. The client fetches the rest and builds the text from its own words. Nothing about data, diffs or logs is in a notification.
- **One event, one notification per person.** Collapse repeats with the same tag (the experiment id), and a newer state replaces an older one.
- **Per person, per device.** Turning notifications off on one device does not touch another. The server keeps each device's push subscription with the person, and deletes it when the browser revokes it or the person signs out there.
- **Notifications are reads.** They change no state, so they are not audited; turning them on or off on a device is not audited either. A notification is sent after the event is committed, from the same live event the app receives ([../change/live-event.md](../change/live-event.md)).
- **A self-hosted server that cannot reach the browsers' push services sends no web push.** Everything else works; say so in Settings, This device.
