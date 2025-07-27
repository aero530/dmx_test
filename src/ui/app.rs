//! Ratatui App
// use core::default;

use defmt::Format;
// use embedded_graphics::prelude::Dimensions;
// use embedded_menu::{interaction::InputAdapterSource, selection_indicator::{style::IndicatorStyle, SelectionIndicatorController}, theme::Theme, Menu};

// use embassy_time::{with_timeout, Duration};

// const TAB_COUNT: usize = 4;

// const TABS: [&str; TAB_COUNT] = ["DMX", "ArtNet", "SmartLED", "PWM"];

#[derive(Default, Clone, Copy, Format)]
pub enum SelectedTab {
    #[default]
    Tab0, // 0
    Tab1,
    Tab2,
    Tab3,
}

impl SelectedTab {
    // pub fn title(&self) -> &str {
    //     let current_index: usize = *self as usize;
    //     TABS[current_index]
    // }
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
    pub fn goto_next_tab(&mut self) {
        self.selected_tab = self.selected_tab.next();
    }

    pub fn goto_previous_tab(&mut self) {
        self.selected_tab = self.selected_tab.previous();
    }


    pub fn current_tab(&mut self) -> SelectedTab {
        self.selected_tab
    }

    // fn render_tabs(&self) {
    //     let titles = TABS;
    //     let selected_tab_index = self.selected_tab as usize;
    //     // let highlight_style = (Color::default(), self.selected_tab.palette().c700);
    //     // Tabs::new(titles)
    //     //     .highlight_style(highlight_style)
    //     //     .select(selected_tab_index)
    //     //     .padding("", "")
    //     //     .divider(" ")
    //     //     .render(area, buf);
    // }
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