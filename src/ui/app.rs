//! Ratatui App
// use core::default;

use defmt::Format;

// use embassy_time::{with_timeout, Duration};

const TABS: [&str; 4] = ["DMX", "ArtNet", "SmartLED", "PWM"];

#[derive(Default, Clone, Copy, Format)]
enum SelectedTab {
    #[default]
    Tab0, // 0
    Tab1,
    Tab2,
    Tab3,
}

impl SelectedTab {
    fn display(&self) -> &str {
        let current_index: usize = *self as usize;
        TABS[current_index]
    }
    fn from_repr(index: usize) -> Self {
        match index {
            0 => Self::Tab0,
            1 => Self::Tab1,
            2 => Self::Tab2,
            3 => Self::Tab3,
            _ => Self::Tab3, // any usize higher than 3 returns the same thing as 3
        }
    }
}

#[derive(Default)]
pub struct App {
    selected_tab: SelectedTab,
}

impl App {
    
    pub fn next_tab(&mut self) {
        self.selected_tab = self.selected_tab.next();
    }

    pub fn previous_tab(&mut self) {
        self.selected_tab = self.selected_tab.previous();
    }

    fn render_tabs(&self) {
        let titles = TABS;
        let selected_tab_index = self.selected_tab as usize;
        // let highlight_style = (Color::default(), self.selected_tab.palette().c700);
        // Tabs::new(titles)
        //     .highlight_style(highlight_style)
        //     .select(selected_tab_index)
        //     .padding("", "")
        //     .divider(" ")
        //     .render(area, buf);
    }
}

impl SelectedTab {
    /// Get the previous tab, if there is no previous tab return the current tab.
    fn previous(self) -> Self {
        let current_index: usize = self as usize;
        let previous_index = current_index.saturating_sub(1);
        Self::from_repr(previous_index)
        // Self::from_repr(previous_index).unwrap_or(self)
    }

    /// Get the next tab, if there is no next tab return the current tab.
    fn next(self) -> Self {
        let current_index = self as usize;
        let next_index = current_index.saturating_add(1);
        Self::from_repr(next_index)
        // Self::from_repr(next_index).unwrap_or(self)
    }
}


impl SelectedTab {
    /// Return tab's name as a styled `Line`
    fn title(self) {
        // let style = Style::new().fg(tailwind::SLATE.c200).bg(self.palette().c900);
        // match self {
        //     SelectedTab::Tab0 => Line::styled(TABS[0], style),
        //     SelectedTab::Tab1 => Line::styled(TABS[1], style),
        //     SelectedTab::Tab2 => Line::styled(TABS[2], style),
        //     SelectedTab::Tab3 => Line::styled(TABS[3], style),
        // }
    }

    fn render_tab0(self) {
        // Paragraph::new("Hello, World!")
        //     .block(self.block())
        //     .render(area, buf);
    }

    fn render_tab1(self) {
        // Paragraph::new("Welcome to the Ratatui tabs example!")
        //     .block(self.block())
        //     .render(area, buf);
    }

    fn render_tab2(self) {
        // Paragraph::new("Look! I'm different than others!")
        //     .block(self.block())
        //     .render(area, buf);
    }

    fn render_tab3(self) {
        // Paragraph::new("I know, these are some basic changes. But I think you got the main idea.")
        //     .block(self.block())
        //     .render(area, buf);
    }

}