# Notifications (target)

A notification tells a person that something needs them or that work they asked about is done. It is never progress chatter.

## What notifies, and whom

| Event | Who gets it | Words (title · line) |
| --- | --- | --- |
| Waiting for you: a card needs approval, or the agent asked a question | whoever started the session, and the editors and owners of its project | Waiting for you · the session title, then what to decide |
| A cap was spent | whoever started the session, and owners | Budget spent · the session title |
| A session finished (Done or Failed) | whoever started the session, only if they asked ("Notify me when this finishes") | Done or Failed · the session title |

The words come from the one catalogue and the glossary ([../design/wording.md](../design/wording.md)); a notification never says something the app would not.

## Where it shows

- **macOS:** native notifications from the Mac app (target), set in its Settings window.
- **Windows and Linux, and any browser:** web push from the web app, installed or in a tab, set per device in Settings, This device. Asked for only from a user action (after starting a session), never on load.
- **The icon badge** counts the sessions that are Waiting for you and clears as they are seen.
- **In the app**, the sidebar and the session page already show the same states; a notification only points there.

## Email

No email, chat or webhook notifications for now. Adding one is a product call. (The monthly report email belonged to the parked judge.)

## Rules

- **Ids only leave the server.** A push payload is a session id and an event kind. The client fetches the rest and builds the text from its own words. Nothing about data, diffs or logs is in a notification.
- **One event, one notification per person.** Collapse repeats with the same tag (the session id), and a newer state replaces an older one.
- **Per person, per device.** Turning notifications off on one device does not touch another. The server keeps each device's push subscription with the person, and deletes it when the browser revokes it or the person signs out there.
- **Notifications are reads.** They change no state, so they are not audited; turning them on or off on a device is not audited either. A notification is sent after the event is committed, from the same live event the app receives ([../change/live-event.md](../change/live-event.md)).
- **A self-hosted server that cannot reach the browsers' push services sends no web push.** Everything else works; say so in Settings, This device.
