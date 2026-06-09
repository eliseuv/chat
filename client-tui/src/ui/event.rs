use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use tokio::sync::mpsc;

/// Represents a high-level application event abstracted from raw terminal input.
///
/// This enum simplifies raw key strokes and terminal events into semantically
/// meaningful actions that the application loop can easily process.
pub enum AppEvent {
    /// An ignored or unhandled terminal event.
    None,
    /// A signal to quit the application (e.g., `Ctrl-Q`).
    Quit,
    /// A cancel/escape event (e.g., `Esc`).
    Cancel,
    /// A standard character input typed by the user.
    InputChar(char),
    /// A backspace keystroke to delete the last character.
    Backspace,
    /// An enter/return keystroke to submit a message.
    Enter,
    /// A terminal resize event requiring a UI redraw.
    Resize,
}

/// A stream wrapper that reads raw terminal events and translates them into `AppEvent`s.
pub struct UiEventStream {
    rx: mpsc::UnboundedReceiver<anyhow::Result<AppEvent>>,
}

impl UiEventStream {
    /// Initializes a new `UiEventStream` connected to the standard terminal event stream.
    pub fn new() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();

        std::thread::spawn(move || {
            loop {
                match event::read() {
                    Ok(Event::Key(key_event)) if key_event.kind == KeyEventKind::Press => {
                        let app_event = if key_event.modifiers.contains(KeyModifiers::CONTROL)
                            && (key_event.code == KeyCode::Char('q') || key_event.code == KeyCode::Char('Q'))
                        {
                            AppEvent::Quit
                        } else {
                            match key_event.code {
                                KeyCode::Char(c) => {
                                    // Ignore character key events that contain Control or Alt modifiers,
                                    // to avoid printing control characters to the buffer.
                                    if key_event.modifiers.contains(KeyModifiers::CONTROL)
                                        || key_event.modifiers.contains(KeyModifiers::ALT)
                                    {
                                        AppEvent::None
                                    } else {
                                        AppEvent::InputChar(c)
                                    }
                                }
                                KeyCode::Backspace => AppEvent::Backspace,
                                KeyCode::Enter => AppEvent::Enter,
                                KeyCode::Esc => AppEvent::Cancel,
                                _ => AppEvent::None,
                            }
                        };
                        if tx.send(Ok(app_event)).is_err() {
                            break;
                        }
                    }
                    Ok(Event::Resize(_, _)) => {
                        if tx.send(Ok(AppEvent::Resize)).is_err() {
                            break;
                        }
                    }
                    Ok(_) => {
                        if tx.send(Ok(AppEvent::None)).is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(Err(e.into()));
                        break;
                    }
                }
            }
        });

        Self { rx }
    }

    /// Asynchronously waits for and returns the next parsed `AppEvent`.
    ///
    /// Intercepts special control sequences like `Ctrl-C` to trigger a `Quit` event.
    pub async fn next(&mut self) -> Option<anyhow::Result<AppEvent>> {
        self.rx.recv().await
    }
}

impl Default for UiEventStream {
    fn default() -> Self {
        Self::new()
    }
}
