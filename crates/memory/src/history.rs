//! `history.jsonl` — portable mirror of an app's event log (spec §B.1): one event per line,
//! in the same JSON format as `opentatu_core::Event`, oldest first.

use opentatu_core::Event;

/// Renders events (oldest first) as JSON Lines.
pub fn render_history(events: &[Event]) -> Result<String, serde_json::Error> {
    let mut out = String::new();
    for event in events {
        out.push_str(&serde_json::to_string(event)?);
        out.push('\n');
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use opentatu_core::{Actor, EventId, EventKind, NewEvent, Slug};

    use super::*;

    #[test]
    fn one_parseable_event_per_line() {
        let event = Event {
            event_id: EventId::new(1),
            content: NewEvent::new(
                EventKind::AppRegistered,
                Actor::Owner,
                "2026-11-12T09:00:00-03:00".parse().unwrap(),
            )
            .with_app(Slug::new("abastevo").unwrap()),
        };
        let text = render_history(&[event.clone(), event.clone()]).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 2);
        assert_eq!(serde_json::from_str::<Event>(lines[0]).unwrap(), event);
    }
}
