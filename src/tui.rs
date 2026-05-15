use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Clear, List, ListItem, ListState, Paragraph, Tabs, Wrap,
    },
    Frame, Terminal,
};
use std::{
    io,
    time::{Duration, Instant},
};
use tokio::sync::mpsc;

use crate::api::{Entry, NewsItem};
use crate::config;

const CATEGORIES: &[(&str, &str)] = &[
    ("agent-framework", "Agent Frameworks"),
    ("typescript", "TypeScript"),
    ("python", "Python"),
    ("llm", "LLMs"),
    ("embedding", "Embeddings"),
    ("vector-db", "Vector DBs"),
    ("rag", "RAG"),
    ("mcp-server", "MCP Servers"),
    ("cli", "CLIs"),
    ("sdk", "SDKs"),
    ("ui", "UI"),
    ("testing", "Testing"),
    ("observability", "Observability"),
    ("deployment", "Deployment"),
    ("search", "Search"),
    ("code-gen", "Code Gen"),
    ("data", "Data"),
    ("voice", "Voice"),
    ("multimodal", "Multimodal"),
    ("fine-tuning", "Fine-Tuning"),
    ("other", "Other"),
];

const KINDS: &[(&str, &str)] = &[
    ("tool", "Tools"),
    ("skill", "Skills"),
    ("mcp", "MCPs"),
];

#[derive(Clone, Copy, PartialEq)]
enum Tab {
    Search,
    News,
    Browse,
}

enum ApiMsg {
    SearchDone(Vec<Entry>),
    NewsDone(Vec<NewsItem>),
    BrowseDone(Vec<Entry>),
    Error(String),
}

struct App {
    tab: Tab,
    token: Option<String>,

    // Search
    search_input: String,
    search_results: Vec<Entry>,
    search_list: ListState,
    search_loading: bool,

    // News
    news_items: Vec<NewsItem>,
    news_list: ListState,
    news_loading: bool,

    // Browse
    kind_idx: usize,
    cat_idx: usize,
    browse_results: Vec<Entry>,
    browse_list: ListState,
    browse_loading: bool,

    // Detail popup
    detail: Option<Entry>,
    detail_news: Option<NewsItem>,

    status: Option<String>,
}

impl App {
    fn new() -> Self {
        let cfg = config::load();
        Self {
            tab: Tab::Search,
            token: cfg.token,
            search_input: String::new(),
            search_results: Vec::new(),
            search_list: ListState::default(),
            search_loading: false,
            news_items: Vec::new(),
            news_list: ListState::default(),
            news_loading: false,
            kind_idx: 0,
            cat_idx: 0,
            browse_results: Vec::new(),
            browse_list: ListState::default(),
            browse_loading: false,
            detail: None,
            detail_news: None,
            status: None,
        }
    }

    fn active_list_len(&self) -> usize {
        match self.tab {
            Tab::Search => self.search_results.len(),
            Tab::News => self.news_items.len(),
            Tab::Browse => self.browse_results.len(),
        }
    }

    fn list_state_mut(&mut self) -> &mut ListState {
        match self.tab {
            Tab::Search => &mut self.search_list,
            Tab::News => &mut self.news_list,
            Tab::Browse => &mut self.browse_list,
        }
    }

    fn move_down(&mut self) {
        let len = self.active_list_len();
        if len == 0 {
            return;
        }
        let state = self.list_state_mut();
        let next = state.selected().map_or(0, |i| (i + 1).min(len - 1));
        state.select(Some(next));
    }

    fn move_up(&mut self) {
        let len = self.active_list_len();
        if len == 0 {
            return;
        }
        let state = self.list_state_mut();
        let next = state.selected().map_or(0, |i| i.saturating_sub(1));
        state.select(Some(next));
    }

    fn open_detail(&mut self) {
        match self.tab {
            Tab::Search => {
                if let Some(i) = self.search_list.selected() {
                    self.detail = self.search_results.get(i).cloned();
                    self.detail_news = None;
                }
            }
            Tab::News => {
                if let Some(i) = self.news_list.selected() {
                    self.detail_news = self.news_items.get(i).cloned();
                    self.detail = None;
                }
            }
            Tab::Browse => {
                if let Some(i) = self.browse_list.selected() {
                    self.detail = self.browse_results.get(i).cloned();
                    self.detail_news = None;
                }
            }
        }
    }

    fn close_detail(&mut self) {
        self.detail = None;
        self.detail_news = None;
    }

    fn has_detail(&self) -> bool {
        self.detail.is_some() || self.detail_news.is_some()
    }

    fn open_link(&self) {
        let url = if let Some(entry) = &self.detail {
            Some(entry.homepage_url.clone())
        } else if let Some(news) = &self.detail_news {
            Some(news.source_url.clone())
        } else {
            None
        };
        if let Some(url) = url {
            let _ = open::that(url);
        }
    }
}

pub async fn run() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_app(&mut terminal).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

async fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    let (tx, mut rx) = mpsc::channel::<ApiMsg>(16);
    let mut app = App::new();

    // Kick off initial news load
    {
        let tx = tx.clone();
        let token = app.token.clone();
        tokio::spawn(async move {
            match crate::api::get_news(7, token.as_deref()).await {
                Ok(items) => { let _ = tx.send(ApiMsg::NewsDone(items)).await; }
                Err(e) => { let _ = tx.send(ApiMsg::Error(e.to_string())).await; }
            }
        });
        app.news_loading = true;
    }

    let tick = Duration::from_millis(200);
    let mut last_tick = Instant::now();
    let mut search_debounce: Option<Instant> = None;

    loop {
        terminal.draw(|f| render(f, &mut app))?;

        // Handle pending API results
        while let Ok(msg) = rx.try_recv() {
            match msg {
                ApiMsg::SearchDone(items) => {
                    app.search_loading = false;
                    app.search_results = items;
                    app.search_list = ListState::default();
                    if !app.search_results.is_empty() {
                        app.search_list.select(Some(0));
                    }
                }
                ApiMsg::NewsDone(items) => {
                    app.news_loading = false;
                    app.news_items = items;
                    app.news_list = ListState::default();
                    if !app.news_items.is_empty() {
                        app.news_list.select(Some(0));
                    }
                }
                ApiMsg::BrowseDone(items) => {
                    app.browse_loading = false;
                    app.browse_results = items;
                    app.browse_list = ListState::default();
                    if !app.browse_results.is_empty() {
                        app.browse_list.select(Some(0));
                    }
                }
                ApiMsg::Error(e) => {
                    app.search_loading = false;
                    app.news_loading = false;
                    app.browse_loading = false;
                    app.status = Some(format!("Error: {}", e));
                }
            }
        }

        // Fire debounced search
        if let Some(debounce_at) = search_debounce {
            if debounce_at.elapsed() >= Duration::from_millis(400) {
                search_debounce = None;
                let q = app.search_input.clone();
                if !q.is_empty() {
                    let tx = tx.clone();
                    let token = app.token.clone();
                    tokio::spawn(async move {
                        let result = if let Some(tok) = &token {
                            crate::api::semantic_search(&q, tok).await
                        } else {
                            crate::api::search_entries(&q, token.as_deref()).await
                        };
                        match result {
                            Ok(items) => { let _ = tx.send(ApiMsg::SearchDone(items)).await; }
                            Err(e) => { let _ = tx.send(ApiMsg::Error(e.to_string())).await; }
                        }
                    });
                    app.search_loading = true;
                    app.status = None;
                }
            }
        }

        let timeout = tick
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                // Quit
                if key.code == KeyCode::Char('q') && !app.has_detail()
                    || (key.code == KeyCode::Char('c')
                        && key.modifiers.contains(KeyModifiers::CONTROL))
                {
                    break;
                }

                // Close detail / escape
                if key.code == KeyCode::Esc {
                    if app.has_detail() {
                        app.close_detail();
                    }
                    continue;
                }

                // Open link in detail
                if key.code == KeyCode::Char('o') && app.has_detail() {
                    app.open_link();
                    continue;
                }

                // Detail navigation
                if app.has_detail() {
                    continue;
                }

                // Tab switching
                if key.code == KeyCode::Tab {
                    app.tab = match app.tab {
                        Tab::Search => Tab::News,
                        Tab::News => Tab::Browse,
                        Tab::Browse => Tab::Search,
                    };
                    if app.tab == Tab::Browse && app.browse_results.is_empty() && !app.browse_loading {
                        fire_browse(&tx, &mut app).await;
                    }
                    continue;
                }
                if key.code == KeyCode::BackTab {
                    app.tab = match app.tab {
                        Tab::Search => Tab::Browse,
                        Tab::News => Tab::Search,
                        Tab::Browse => Tab::News,
                    };
                    continue;
                }
                if let KeyCode::Char(c @ '1'..='3') = key.code {
                    app.tab = match c {
                        '1' => Tab::Search,
                        '2' => Tab::News,
                        '3' => Tab::Browse,
                        _ => app.tab,
                    };
                    if app.tab == Tab::Browse && app.browse_results.is_empty() && !app.browse_loading {
                        fire_browse(&tx, &mut app).await;
                    }
                    continue;
                }

                match app.tab {
                    Tab::Search => match key.code {
                        KeyCode::Down => app.move_down(),
                        KeyCode::Up => app.move_up(),
                        KeyCode::Enter => app.open_detail(),
                        KeyCode::Backspace => {
                            app.search_input.pop();
                            if app.search_input.is_empty() {
                                app.search_results.clear();
                                app.search_list = ListState::default();
                            } else {
                                search_debounce = Some(Instant::now());
                            }
                        }
                        KeyCode::Char(c) => {
                            app.search_input.push(c);
                            search_debounce = Some(Instant::now());
                        }
                        _ => {}
                    },

                    Tab::News => match key.code {
                        KeyCode::Down | KeyCode::Char('j') => app.move_down(),
                        KeyCode::Up | KeyCode::Char('k') => app.move_up(),
                        KeyCode::Enter => app.open_detail(),
                        _ => {}
                    },

                    Tab::Browse => match key.code {
                        KeyCode::Left | KeyCode::Char('h') => {
                            if app.kind_idx > 0 {
                                app.kind_idx -= 1;
                                fire_browse(&tx, &mut app).await;
                            }
                        }
                        KeyCode::Right | KeyCode::Char('l') => {
                            if app.kind_idx + 1 < KINDS.len() {
                                app.kind_idx += 1;
                                fire_browse(&tx, &mut app).await;
                            }
                        }
                        KeyCode::Char('[') => {
                            if app.cat_idx > 0 {
                                app.cat_idx -= 1;
                                fire_browse(&tx, &mut app).await;
                            }
                        }
                        KeyCode::Char(']') => {
                            if app.cat_idx + 1 < CATEGORIES.len() {
                                app.cat_idx += 1;
                                fire_browse(&tx, &mut app).await;
                            }
                        }
                        KeyCode::Down | KeyCode::Char('j') => app.move_down(),
                        KeyCode::Up | KeyCode::Char('k') => app.move_up(),
                        KeyCode::Enter => app.open_detail(),
                        _ => {}
                    },
                }
            }
        }

        if last_tick.elapsed() >= tick {
            last_tick = Instant::now();
        }
    }

    Ok(())
}

async fn fire_browse(tx: &mpsc::Sender<ApiMsg>, app: &mut App) {
    let kind = KINDS[app.kind_idx].0.to_string();
    let cat = CATEGORIES[app.cat_idx].0.to_string();
    let token = app.token.clone();
    let tx = tx.clone();
    tokio::spawn(async move {
        match crate::api::list_by_category(&kind, &cat, token.as_deref()).await {
            Ok(items) => { let _ = tx.send(ApiMsg::BrowseDone(items)).await; }
            Err(e) => { let _ = tx.send(ApiMsg::Error(e.to_string())).await; }
        }
    });
    app.browse_loading = true;
    app.status = None;
}

fn render(f: &mut Frame, app: &mut App) {
    let area = f.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // header/tabs
            Constraint::Min(0),    // content
            Constraint::Length(1), // status bar
        ])
        .split(area);

    render_header(f, app, chunks[0]);
    render_content(f, app, chunks[1]);
    render_status(f, app, chunks[2]);

    if app.has_detail() {
        render_detail(f, app, area);
    }
}

fn render_header(f: &mut Frame, app: &App, area: Rect) {
    let tab_titles: Vec<Line> = vec![
        Line::from(Span::raw("1 Search")),
        Line::from(Span::raw("2 News")),
        Line::from(Span::raw("3 Browse")),
    ];
    let selected = match app.tab {
        Tab::Search => 0,
        Tab::News => 1,
        Tab::Browse => 2,
    };
    let tabs = Tabs::new(tab_titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Update Night "),
        )
        .select(selected)
        .style(Style::default().fg(Color::White))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );
    f.render_widget(tabs, area);
}

fn render_content(f: &mut Frame, app: &mut App, area: Rect) {
    match app.tab {
        Tab::Search => render_search(f, app, area),
        Tab::News => render_news(f, app, area),
        Tab::Browse => render_browse(f, app, area),
    }
}

fn render_search(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    let auth_hint = if app.token.is_some() {
        " (semantic)"
    } else {
        " (text · run `un login` for semantic)"
    };
    let loading_suffix = if app.search_loading { " ⟳" } else { "" };
    let input = Paragraph::new(app.search_input.as_str())
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" Search{}{} ", auth_hint, loading_suffix)),
        )
        .style(Style::default().fg(Color::White));
    f.render_widget(input, chunks[0]);

    let items: Vec<ListItem> = app
        .search_results
        .iter()
        .map(|e| entry_list_item(e))
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(" Results "))
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    f.render_stateful_widget(list, chunks[1], &mut app.search_list);
}

fn render_news(f: &mut Frame, app: &mut App, area: Rect) {
    let loading_suffix = if app.news_loading { " ⟳" } else { "" };
    let items: Vec<ListItem> = app
        .news_items
        .iter()
        .map(|n| {
            let date = n.posted_at.get(..10).unwrap_or("");
            let topics = n.topics.join(", ");
            ListItem::new(vec![
                Line::from(vec![
                    Span::styled(&n.title, Style::default().add_modifier(Modifier::BOLD)),
                ]),
                Line::from(vec![
                    Span::styled(date, Style::default().fg(Color::DarkGray)),
                    Span::raw("  "),
                    Span::styled(topics, Style::default().fg(Color::Cyan)),
                ]),
            ])
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" News (last 7 days){} ", loading_suffix)),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    f.render_stateful_widget(list, area, &mut app.news_list);
}

fn render_browse(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    let kind_label = KINDS[app.kind_idx].1;
    let cat_label = CATEGORIES[app.cat_idx].1;
    let loading_suffix = if app.browse_loading { " ⟳" } else { "" };

    let controls = Paragraph::new(format!(
        " Kind: {} (← →)   Category: {} ([ ]){} ",
        kind_label, cat_label, loading_suffix
    ))
    .block(Block::default().borders(Borders::ALL).title(" Browse "))
    .style(Style::default().fg(Color::White));
    f.render_widget(controls, chunks[0]);

    let items: Vec<ListItem> = app
        .browse_results
        .iter()
        .map(|e| entry_list_item(e))
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(" Entries "))
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    f.render_stateful_widget(list, chunks[1], &mut app.browse_list);
}

fn entry_list_item(e: &Entry) -> ListItem<'_> {
    let pricing = e.pricing.as_deref().unwrap_or("—");
    let cats = e.categories.get(0).map_or("", |s| s.as_str());
    ListItem::new(vec![
        Line::from(vec![
            Span::styled(&e.name, Style::default().add_modifier(Modifier::BOLD)),
            Span::raw("  "),
            Span::styled(pricing, Style::default().fg(Color::Green)),
        ]),
        Line::from(vec![
            Span::styled(&e.tagline, Style::default().fg(Color::Gray)),
            Span::raw("  "),
            Span::styled(cats, Style::default().fg(Color::Cyan)),
        ]),
    ])
}

fn render_status(f: &mut Frame, app: &App, area: Rect) {
    let auth_status = if app.token.is_some() {
        Span::styled("● authed", Style::default().fg(Color::Green))
    } else {
        Span::styled("○ not logged in · run `un login`", Style::default().fg(Color::DarkGray))
    };

    let msg = app
        .status
        .as_deref()
        .unwrap_or("Tab/1-3 switch · ↑↓/jk navigate · Enter open · o open URL · q quit");

    let help = Span::styled(msg, Style::default().fg(Color::DarkGray));

    let line = Line::from(vec![auth_status, Span::raw("  "), help]);
    f.render_widget(Paragraph::new(line), area);
}

fn render_detail(f: &mut Frame, app: &App, area: Rect) {
    let popup_area = centered_rect(80, 80, area);
    f.render_widget(Clear, popup_area);

    if let Some(entry) = &app.detail {
        let mut lines: Vec<Line> = vec![
            Line::from(Span::styled(
                &entry.name,
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                &entry.tagline,
                Style::default().fg(Color::White),
            )),
            Line::from(""),
        ];

        if let Some(desc) = &entry.description {
            for chunk in desc.chars().collect::<Vec<_>>().chunks(popup_area.width as usize - 6) {
                lines.push(Line::from(chunk.iter().collect::<String>()));
            }
            lines.push(Line::from(""));
        }

        if let Some(snippet) = &entry.install_snippet {
            lines.push(Line::from(Span::styled(
                "Install:",
                Style::default().add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(Span::styled(
                snippet.as_str(),
                Style::default().fg(Color::Cyan),
            )));
            lines.push(Line::from(""));
        }

        let pricing_str = entry.pricing.as_deref().unwrap_or("—");
        lines.push(Line::from(vec![
            Span::styled("Pricing: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(pricing_str, Style::default().fg(Color::Green)),
        ]));

        lines.push(Line::from(vec![
            Span::styled("URL: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(&entry.homepage_url, Style::default().fg(Color::Blue)),
        ]));

        let para = Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Detail · Esc close · o open URL "),
            )
            .wrap(Wrap { trim: true });
        f.render_widget(para, popup_area);
    } else if let Some(news) = &app.detail_news {
        let date = news.posted_at.get(..10).unwrap_or("");
        let lines = vec![
            Line::from(Span::styled(
                &news.title,
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(vec![
                Span::styled(date, Style::default().fg(Color::DarkGray)),
                Span::raw("  "),
                Span::styled(&news.source_name, Style::default().fg(Color::Cyan)),
            ]),
            Line::from(""),
            Line::from(Span::raw(&news.summary)),
            Line::from(""),
            Line::from(vec![
                Span::styled("Topics: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(news.topics.join(", "), Style::default().fg(Color::Cyan)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Source: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(&news.source_url, Style::default().fg(Color::Blue)),
            ]),
        ];
        let para = Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" News · Esc close · o open URL "),
            )
            .wrap(Wrap { trim: true });
        f.render_widget(para, popup_area);
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
