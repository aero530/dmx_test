//! Host-side Ratatui console for the DMX interface.
//!
//! Connects to the device's console CDC serial port (the second of the
//! serial ports exposed by the composite USB device) and speaks its line
//! protocol (`get` / `set` / `dmx` / `info`).
//!
//! Usage:
//!   dmx_console <PORT>      e.g. `dmx_console COM5` or `/dev/ttyACM1`
//!   dmx_console             list available ports (device ports are marked)
//!
//! Keys: Tab switch view, Up/Down select or scroll, Enter edit/apply,
//! Esc cancel edit, r refresh, q quit.

use std::io::{self, Read, Write};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::ExecutableCommand;

use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Layout, Margin};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Row, Table, TableState, Tabs};
use ratatui::{Frame, Terminal};

/// VID/PID of the device's composite USB port (see nucleo/src/usb_device.rs).
const DEVICE_VID: u16 = 0xc0de;
const DEVICE_PID: u16 = 0xdcaf;

const SETTINGS_REFRESH: Duration = Duration::from_secs(2);
const DMX_REFRESH: Duration = Duration::from_millis(250);

#[derive(Clone, Copy, PartialEq)]
enum View {
    Settings,
    DmxMonitor,
}

struct App {
    port: Box<dyn serialport::SerialPort>,
    incoming: mpsc::Receiver<String>,

    view: View,
    settings: Vec<(String, String)>,
    table_state: TableState,
    /// Some(buffer) while editing the selected setting
    edit: Option<String>,

    dmx: [u8; 512],
    dmx_scroll: u16,

    status: String,
    last_settings_poll: Instant,
    last_dmx_poll: Instant,
}

impl App {
    fn new(port: Box<dyn serialport::SerialPort>, incoming: mpsc::Receiver<String>) -> Self {
        let mut table_state = TableState::default();
        table_state.select(Some(0));
        Self {
            port,
            incoming,
            view: View::Settings,
            settings: Vec::new(),
            table_state,
            edit: None,
            dmx: [0; 512],
            dmx_scroll: 0,
            status: String::from("connecting..."),
            last_settings_poll: Instant::now() - SETTINGS_REFRESH,
            last_dmx_poll: Instant::now(),
        }
    }

    fn send(&mut self, command: &str) {
        let _ = self.port.write_all(command.as_bytes());
        let _ = self.port.write_all(b"\n");
    }

    /// Apply one line received from the device.
    fn handle_line(&mut self, line: &str) {
        let line = line.trim();
        if line == "ok" {
            return;
        }
        if let Some(err) = line.strip_prefix("err ") {
            self.status = format!("device error: {err}");
            return;
        }
        if let Some(rest) = line.strip_prefix("dmx ") {
            // "dmx <start> v v v ..."
            let mut parts = rest.split_whitespace();
            if let Some(start) = parts.next().and_then(|s| s.parse::<usize>().ok()) {
                for (i, value) in parts.enumerate() {
                    if let (Ok(v), Some(slot)) = (value.parse::<u8>(), self.dmx.get_mut(start - 1 + i)) {
                        *slot = v;
                    }
                }
            }
            return;
        }
        if let Some((key, value)) = line.split_once('=') {
            match self.settings.iter_mut().find(|(k, _)| k == key) {
                Some(entry) => entry.1 = value.to_string(),
                None => self.settings.push((key.to_string(), value.to_string())),
            }
            self.status = String::from("connected");
        }
    }

    fn poll(&mut self) {
        while let Ok(line) = self.incoming.try_recv() {
            self.handle_line(&line);
        }
        if self.view == View::Settings && self.last_settings_poll.elapsed() >= SETTINGS_REFRESH {
            self.last_settings_poll = Instant::now();
            self.send("get");
        }
        if self.view == View::DmxMonitor && self.last_dmx_poll.elapsed() >= DMX_REFRESH {
            self.last_dmx_poll = Instant::now();
            self.send("dmx 1 512");
        }
    }

    /// Returns false when the app should exit.
    fn handle_key(&mut self, code: KeyCode, modifiers: KeyModifiers) -> bool {
        // Editing a value: capture text input first
        if let Some(buffer) = &mut self.edit {
            match code {
                KeyCode::Esc => self.edit = None,
                KeyCode::Backspace => {
                    buffer.pop();
                }
                KeyCode::Enter => {
                    let value = self.edit.take().unwrap_or_default();
                    if let Some(index) = self.table_state.selected() {
                        if let Some((key, _)) = self.settings.get(index) {
                            let command = format!("set {key} {value}");
                            self.status = format!("> {command}");
                            self.send(&command);
                            self.send("get");
                        }
                    }
                }
                KeyCode::Char(c) => buffer.push(c),
                _ => {}
            }
            return true;
        }

        match code {
            KeyCode::Char('q') => return false,
            KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => return false,
            KeyCode::Tab => {
                self.view = if self.view == View::Settings { View::DmxMonitor } else { View::Settings };
            }
            KeyCode::Char('r') => match self.view {
                View::Settings => self.send("get"),
                View::DmxMonitor => self.send("dmx 1 512"),
            },
            KeyCode::Up => match self.view {
                View::Settings => {
                    let i = self.table_state.selected().unwrap_or(0);
                    self.table_state.select(Some(i.saturating_sub(1)));
                }
                View::DmxMonitor => self.dmx_scroll = self.dmx_scroll.saturating_sub(1),
            },
            KeyCode::Down => match self.view {
                View::Settings => {
                    let i = self.table_state.selected().unwrap_or(0);
                    if i + 1 < self.settings.len() {
                        self.table_state.select(Some(i + 1));
                    }
                }
                View::DmxMonitor => self.dmx_scroll = (self.dmx_scroll + 1).min(31),
            },
            KeyCode::Enter => {
                if self.view == View::Settings {
                    if let Some((_, value)) = self.table_state.selected().and_then(|i| self.settings.get(i)) {
                        self.edit = Some(value.clone());
                    }
                }
            }
            _ => {}
        }
        true
    }

    fn render(&mut self, frame: &mut Frame) {
        let [tab_area, body_area, status_area] =
            Layout::vertical([Constraint::Length(1), Constraint::Fill(1), Constraint::Length(1)]).areas(frame.area());

        let tabs = Tabs::new(["Settings", "DMX Monitor"])
            .select(match self.view {
                View::Settings => 0,
                View::DmxMonitor => 1,
            })
            .style(Style::new().fg(Color::Gray))
            .highlight_style(Style::new().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD));
        frame.render_widget(tabs, tab_area);

        match self.view {
            View::Settings => self.render_settings(frame, body_area),
            View::DmxMonitor => self.render_dmx(frame, body_area),
        }

        let hint = if self.edit.is_some() {
            "  type value, Enter apply, Esc cancel"
        } else {
            "  Tab view | \u{2191}\u{2193} select | Enter edit | r refresh | q quit"
        };
        let status = Line::from(vec![
            Span::styled(self.status.clone(), Style::new().fg(Color::Yellow)),
            Span::styled(hint, Style::new().fg(Color::DarkGray)),
        ]);
        frame.render_widget(Paragraph::new(status), status_area);
    }

    fn render_settings(&mut self, frame: &mut Frame, area: ratatui::layout::Rect) {
        let selected = self.table_state.selected().unwrap_or(0);
        let rows: Vec<Row> = self
            .settings
            .iter()
            .enumerate()
            .map(|(i, (key, value))| {
                let shown = match (&self.edit, i == selected) {
                    (Some(buffer), true) => format!("{buffer}_"),
                    _ => value.clone(),
                };
                let style = match (&self.edit, i == selected) {
                    (Some(_), true) => Style::new().fg(Color::Black).bg(Color::Yellow),
                    (None, true) => Style::new().add_modifier(Modifier::BOLD),
                    _ => Style::new(),
                };
                Row::new(vec![key.clone(), shown]).style(style)
            })
            .collect();

        let table = Table::new(rows, [Constraint::Length(20), Constraint::Fill(1)])
            .block(Block::default().borders(Borders::ALL).title(" Settings "))
            .row_highlight_style(Style::new().bg(Color::Rgb(40, 40, 60)));
        frame.render_stateful_widget(table, area, &mut self.table_state);
    }

    fn render_dmx(&mut self, frame: &mut Frame, area: ratatui::layout::Rect) {
        let block = Block::default().borders(Borders::ALL).title(" DMX Channels (1-512) ");
        let inner = area.inner(Margin::new(1, 1));
        frame.render_widget(block, area);

        let mut lines: Vec<Line> = Vec::new();
        for row in 0..32 {
            let start = row * 16;
            let mut spans: Vec<Span> = vec![Span::styled(format!("{:>3} \u{2502} ", start + 1), Style::new().fg(Color::DarkGray))];
            for ch in start..start + 16 {
                let v = self.dmx[ch];
                let style = if v == 0 {
                    Style::new().fg(Color::DarkGray)
                } else {
                    // Shade from dim to bright green with level
                    Style::new().fg(Color::Rgb(60 + (v / 2), 255 - (v / 4), 60))
                };
                spans.push(Span::styled(format!("{v:>3} "), style));
            }
            lines.push(Line::from(spans));
        }

        let paragraph = Paragraph::new(lines).scroll((self.dmx_scroll, 0));
        frame.render_widget(paragraph, inner);
    }
}

/// Reader thread: collect bytes from the serial port and emit whole lines.
fn spawn_reader(mut port: Box<dyn serialport::SerialPort>, tx: mpsc::Sender<String>) {
    std::thread::spawn(move || {
        let mut pending: Vec<u8> = Vec::new();
        let mut buf = [0_u8; 256];
        loop {
            match port.read(&mut buf) {
                Ok(0) => {}
                Ok(n) => {
                    pending.extend_from_slice(&buf[..n]);
                    while let Some(pos) = pending.iter().position(|&b| b == b'\n') {
                        let line: Vec<u8> = pending.drain(..=pos).collect();
                        if let Ok(text) = String::from_utf8(line) {
                            if tx.send(text).is_err() {
                                return;
                            }
                        }
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::TimedOut => {}
                Err(_) => return,
            }
        }
    });
}

fn list_ports() {
    println!("Available serial ports:");
    match serialport::available_ports() {
        Err(e) => println!("  (error: {e})"),
        Ok(ports) if ports.is_empty() => println!("  (none)"),
        Ok(ports) => {
            for p in ports {
                match &p.port_type {
                    serialport::SerialPortType::UsbPort(usb) if usb.vid == DEVICE_VID && usb.pid == DEVICE_PID => {
                        println!("  {}  <- DMX interface (console is the 2nd of its ports)", p.port_name);
                    }
                    _ => println!("  {}", p.port_name),
                }
            }
        }
    }
    println!("\nUsage: dmx_console <PORT>");
}

fn main() -> io::Result<()> {
    let Some(port_name) = std::env::args().nth(1) else {
        list_ports();
        return Ok(());
    };

    let port = serialport::new(&port_name, 115_200)
        .timeout(Duration::from_millis(50))
        .open()
        .map_err(|e| io::Error::other(format!("cannot open {port_name}: {e}")))?;

    let reader_port = port.try_clone().map_err(io::Error::other)?;
    let (line_tx, line_rx) = mpsc::channel();
    spawn_reader(reader_port, line_tx);

    let mut app = App::new(port, line_rx);
    app.send("get");

    enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    let result = run(&mut terminal, &mut app);

    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;
    result
}

fn run(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> io::Result<()> {
    loop {
        app.poll();
        terminal.draw(|frame| app.render(frame))?;

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press && !app.handle_key(key.code, key.modifiers) {
                    return Ok(());
                }
            }
        }
    }
}
