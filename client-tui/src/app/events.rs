use crate::app::core::ChatApp;
use crate::app::state::State;
use crate::ui::AppEvent;
use server::protocol::MessageContent;

impl ChatApp {
    pub async fn handle_event(&mut self, event: AppEvent) -> anyhow::Result<()> {
        if self.is_confirming_quit {
            match event {
                AppEvent::InputChar('y') | AppEvent::InputChar('Y') => {
                    self.state = State::Quit;
                }
                AppEvent::InputChar('n') | AppEvent::InputChar('N') | AppEvent::Cancel => {
                    self.is_confirming_quit = false;
                    self.draw()?;
                }
                AppEvent::Resize => {
                    self.draw()?;
                }
                _ => {}
            }
            return Ok(());
        }

        if !matches!(event, AppEvent::Tab | AppEvent::None | AppEvent::Resize) {
            self.autocomplete_state = None;
        }

        match event {
            AppEvent::None => {}

            AppEvent::Quit => {
                self.is_confirming_quit = true;
                self.draw()?;
            }

            AppEvent::Cancel => {}

            AppEvent::InputChar(c) => {
                self.input_buffer.push(c);
                self.draw()?;
            }

            AppEvent::Backspace => {
                self.input_buffer.pop();
                self.draw()?;
            }

            AppEvent::Tab => {
                self.handle_autocomplete();
                self.draw()?;
            }

            AppEvent::Enter => {
                if !self.input_buffer.is_empty() {
                    let text = std::mem::take(&mut self.input_buffer);
                    match self.state {
                        State::Login => {
                            self.history.own_username = Some(text.clone());
                            if let Err(e) = self.send_login(text).await {
                                log::error!("Failed to send login: {}", e);
                            }
                        }
                        State::ChatRoom => {
                            if let Err(e) = self.send_message(MessageContent::Text(text)).await {
                                log::error!("Failed to send message: {}", e);
                            }
                        }
                        _ => {}
                    }
                    self.draw()?;
                }
            }

            AppEvent::Resize => {
                self.draw()?;
            }
        }
        Ok(())
    }

    pub fn handle_autocomplete(&mut self) {
        if self.state != State::ChatRoom {
            return;
        }

        let (prefix, match_idx, start_idx) = match self.autocomplete_state.take() {
            Some(state) => state,
            None => {
                if self.input_buffer.is_empty() || self.input_buffer.ends_with(|c: char| c.is_whitespace()) {
                    return;
                }

                let last_space_idx = self.input_buffer.rfind(|c: char| c.is_whitespace());
                let word_start = last_space_idx.map(|i| i + 1).unwrap_or(0);
                let last_word = &self.input_buffer[word_start..];
                
                if !last_word.starts_with('@') {
                    return;
                }
                
                let prefix = last_word[1..].to_string();
                (prefix, 0, word_start)
            }
        };

        let own_username = self.history.own_username.as_deref().unwrap_or("");
        let mut matches: Vec<String> = self.history.active_usernames
            .iter()
            .filter(|u| u.to_lowercase().starts_with(&prefix.to_lowercase()) && u.as_str() != own_username)
            .cloned()
            .collect();

        if matches.is_empty() {
            return;
        }

        matches.sort();

        let suggestion = &matches[match_idx % matches.len()];

        self.input_buffer.truncate(start_idx);
        self.input_buffer.push_str(&format!("@{} ", suggestion));

        self.autocomplete_state = Some((prefix, match_idx + 1, start_idx));
    }
}
