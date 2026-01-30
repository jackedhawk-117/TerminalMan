use anyhow::Result;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

use super::Pane;
use crate::{
    ctx::Ctx,
    shared::{
        keys::ActionEvent,
        macros::{status_error, status_info},
    },
    ui::{
        input::{BufferId, InputResultEvent},
    },
};

#[derive(Debug)]
pub struct DownloaderPane {
    input_id: BufferId,
}

impl DownloaderPane {
    pub fn new(ctx: &Ctx) -> Self {
        let input_id = BufferId::new();
        ctx.input.create_buffer(input_id, None);
        Self { input_id }
    }
}

impl Pane for DownloaderPane {
    fn render(&mut self, frame: &mut ratatui::Frame, area: Rect, ctx: &Ctx) -> Result<()> {
        let chunks = Layout::default()
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(area);

        let input_area = chunks[0];
        let list_area = chunks[1];

        // Render Input
        let value = ctx.input.value(self.input_id);
        let style = if ctx.input.is_active(self.input_id) {
            Style::default().fg(ratatui::style::Color::Yellow)
        } else {
            Style::default()
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Paste YouTube/SoundCloud URL (Press 'i' to edit, 'Enter' to download) ");
        
        let mut text = value.to_string();
        if ctx.input.is_active(self.input_id) {
            text.push_str("█"); // Fake cursor
        }

        let paragraph = Paragraph::new(text).block(block).style(style);
        frame.render_widget(paragraph, input_area);

        // Render Downloads List (from YtDlpManager)
        let items: Vec<ListItem> = ctx.ytdlp_manager.map_values(|item| {
            let (status, color) = match &item.state {
                crate::shared::ytdlp::DownloadState::Queued => ("Queued".to_string(), ratatui::style::Color::Gray),
                crate::shared::ytdlp::DownloadState::Downloading { started_at } => {
                    let elapsed = started_at.elapsed().unwrap_or_default();
                    let secs = elapsed.as_secs();
                    let time_str = if secs < 60 {
                        format!("{}s", secs)
                    } else {
                        format!("{}m {}s", secs / 60, secs % 60)
                    };
                    (format!("Downloading... ({})", time_str), ratatui::style::Color::Yellow)
                }
                crate::shared::ytdlp::DownloadState::Completed { .. } => ("Completed".to_string(), ratatui::style::Color::Green),
                crate::shared::ytdlp::DownloadState::AlreadyDownloaded { .. } => ("Already Downloaded".to_string(), ratatui::style::Color::Green),
                crate::shared::ytdlp::DownloadState::Failed { logs } => {
                    // Try to find a meaningful error message from the end of the logs
                    let error = logs
                        .iter()
                        .rev()
                        .find(|line| !line.trim().is_empty() && !line.contains("yt-dlp exited with code"))
                        .cloned()
                        .unwrap_or_else(|| "Unknown error".to_string());
                    (format!("Failed: {}", error), ratatui::style::Color::Red)
                }
                crate::shared::ytdlp::DownloadState::Canceled => ("Canceled".to_string(), ratatui::style::Color::Gray),
            };
            
            let line = Line::from(vec![
                Span::styled(format!("[{}] ", status), Style::default().fg(color)),
                Span::raw(format!("{} : {}", item.inner.id, item.inner.kind)),
            ]);
            ListItem::new(line)
        });

        let list_block = Block::default().borders(Borders::ALL).title(" Recent Downloads ");
        let list = List::new(items).block(list_block);
        frame.render_widget(list, list_area);

        Ok(())
    }

    fn handle_action(&mut self, event: &mut ActionEvent, ctx: &mut Ctx) -> Result<()> {
        // If in insert mode, common actions might be claimed by handle_insert_mode
        // enabling insert mode:
        if let Some(action) = event.claim_common() {
             match action {
                crate::config::keys::CommonAction::FocusInput => {
                    ctx.input.insert_mode(self.input_id);
                    ctx.render()?;
                }
                _ => {}
             }
        }
        Ok(())
    }

    fn handle_insert_mode(&mut self, kind: InputResultEvent, ctx: &mut Ctx) -> Result<()> {
        match kind {
            InputResultEvent::Confirm => {
                let url = ctx.input.value(self.input_id).trim().to_owned();
                if !url.is_empty() {
                    match ctx.ytdlp_manager.download_url(&url, None) {
                        Ok(_) => {
                            status_info!("Download started for: {}", url);
                            ctx.input.clear_buffer(self.input_id);
                        }
                        Err(e) => {
                            status_error!("Failed to start download: {}", e);
                        }
                    }
                }
                ctx.input.normal_mode();
                ctx.render()?;
            }
            InputResultEvent::Cancel => {
                ctx.input.normal_mode();
                ctx.render()?;
            }
            _ => {}
        }
        Ok(())
    }
}
