use anyhow::{Context, Result};
use clap::Parser;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use redis::{Client, Commands};
use std::collections::HashMap;
use std::io;
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans, Text},
    widgets::{Block, Borders, Cell, List, ListItem, ListState, Paragraph, Row, Table},
    Frame, Terminal,
};

// 应用状态
struct App {
    redis_client: Option<Client>,
    redis_connection: Option<redis::Connection>,
    keys: Vec<String>,
    search_match_keys: Vec<String>,
    selected_key: usize,
    key_details: HashMap<String, KeyDetails>,
    status: String,
    search_query: String,
    show_details: bool,
}

// 键详情结构
struct KeyDetails {
    key_type: String,
    ttl: i64,
    value: String,
    hash_fields: Option<HashMap<String, String>>,
}

impl App {
    fn new() -> Self {
        Self {
            redis_client: None,
            redis_connection: None,
            keys: Vec::new(),
            search_match_keys: Vec::new(),
            selected_key: 0,
            key_details: HashMap::new(),
            status: String::from("Not connected to Redis server"),
            search_query: String::new(),
            show_details: false,
        }
    }

    // 连接到Redis
    fn connect_redis(&mut self, addr: &str) -> Result<()> {
        let client = Client::open(addr)?;
        let conn = client.get_connection()?;
        self.redis_client = Some(client);
        self.redis_connection = Some(conn);
        self.status = format!("Connect to Redis server: {}", addr);
        self.load_keys()?;
        Ok(())
    }

    // 加载所有键
    fn load_keys(&mut self) -> Result<()> {
        if let Some(conn) = &mut self.redis_connection {
            let keys: Vec<String> = conn.keys("*")?;
            self.keys = keys;
            self.status = format!("Find {} keys", self.keys.len());
            self.key_details.clear();
            self.selected_key = 0;
        }
        Ok(())
    }

    // 获取键详情
    fn load_key_details(&mut self, key: &str) -> Result<()> {
        if let Some(conn) = &mut self.redis_connection {
            // 获取键类型
            let key_type: String = redis::cmd("TYPE").arg(key).query(conn)?;

            // 获取TTL
            let ttl: i64 = conn.ttl(key)?;

            // 根据类型获取值
            let (value, hash_fields) = match key_type.as_str() {
                "string" => {
                    let value: String = conn.get(key)?;
                    (value, None)
                }
                "hash" => {
                    let fields: HashMap<String, String> = conn.hgetall(key)?;
                    let value = format!("Hash type, {} fields", fields.len());
                    (value, Some(fields))
                }
                "list" => {
                    let len: usize = conn.llen(key)?;
                    let value = format!("List type, {} elements", len);
                    (value, None)
                }
                "set" => {
                    let len: usize = conn.scard(key)?;
                    let value = format!("Set type, {} elements", len);
                    (value, None)
                }
                "zset" => {
                    let len: usize = conn.zcard(key)?;
                    let value = format!("ZSet type, {} elements", len);
                    (value, None)
                }
                _ => (String::from(format!("Unknown type {}", key_type)), None),
            };

            self.key_details.insert(
                key.to_string(),
                KeyDetails {
                    key_type: key_type.clone(),
                    ttl,
                    value,
                    hash_fields,
                },
            );
        }
        Ok(())
    }

    // 处理按键事件
    fn handle_key_events(&mut self, key: KeyCode) -> Result<bool> {
        match key {
            KeyCode::Char('Q') => return Ok(true),
            KeyCode::Char('C') => {
                if let Event::Key(key) = event::read()? {
                    if key.modifiers.contains(event::KeyModifiers::CONTROL) {
                        return Ok(true);
                    }
                }
            }
            KeyCode::Char('R') => {
                self.load_keys()?;
                self.status = "Keys list refreshed".to_string();
            }
            KeyCode::Enter => {
                if let Some(key) = self.keys.get(self.selected_key) {
                    self.load_key_details(&key.clone())?;
                    self.show_details = true;
                }
            }
            KeyCode::Esc => {
                self.show_details = false;
            }
            KeyCode::Up => {
                if !self.keys.is_empty() {
                    let selected = self.selected_key;
                    self.selected_key = if selected == 0 {
                        let keys = if self.search_query.is_empty() {
                            &self.keys
                        } else {
                            &self.search_match_keys
                        };
                        keys.len() - 1
                    } else {
                        selected - 1
                    };
                }
            }
            KeyCode::Down => {
                if !self.keys.is_empty() {
                    let selected = self.selected_key;
                    let keys = if self.search_query.is_empty() {
                        &self.keys
                    } else {
                        &self.search_match_keys
                    };
                    self.selected_key = if selected == keys.len() - 1 {
                        0
                    } else {
                        selected + 1
                    };
                }
            }
            KeyCode::Char(c) => {
                if !self.show_details {
                    self.search_query.push(c);
                    self.filtered_keys();
                    self.selected_key = 0;
                }
            }
            KeyCode::Backspace => {
                if !self.show_details {
                    self.search_query.pop();
                    if !self.search_query.is_empty() {
                        self.filtered_keys();
                    }
                    self.selected_key = 0;
                }
            }
            _ => {}
        }
        Ok(false)
    }
    /// Get filtered keys list
    fn filtered_keys(&mut self) {
        if self.search_query.is_empty() {
            return;
        }
        self.search_match_keys = self
            .keys
            .iter()
            .filter(|key| {
                key.to_lowercase()
                    .contains(&self.search_query.to_lowercase())
            })
            .cloned()
            .collect();
    }
}

// 初始化终端
fn init_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

// 恢复终端
fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}

// 渲染界面
fn render<B: Backend>(f: &mut Frame<B>, app: &App) {
    let size = f.size();

    // 主布局
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(size);

    // 顶部状态栏
    let status_bar = Paragraph::new(app.status.clone())
        .style(Style::default().bg(Color::Blue).fg(Color::White))
        .block(Block::default().borders(Borders::NONE));
    f.render_widget(status_bar, chunks[1]);

    if app.show_details {
        // 显示键详情
        render_key_details(f, app, chunks[0]);
    } else {
        // 显示键列表
        render_key_list(f, app, chunks[0]);
    }

    // 底部帮助栏
    let help_text = Spans::from(vec![
        Span::raw("KeyMap: "),
        Span::styled("Q ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw("Quit "),
        Span::styled("R ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw("Refresh "),
        Span::styled("Enter ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw("View Details "),
        Span::styled("ESC ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw("Back "),
    ]);
    let help_bar = Paragraph::new(help_text)
        .style(Style::default().bg(Color::DarkGray).fg(Color::White))
        .block(Block::default().borders(Borders::NONE));
    f.render_widget(help_bar, chunks[2]);
}

// 渲染键列表
fn render_key_list<B: Backend>(f: &mut Frame<B>, app: &App, area: tui::layout::Rect) {
    // 分割区域为搜索框和列表
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // 搜索框高度
            Constraint::Min(1),    // 列表高度
        ])
        .split(area);

    // 渲染搜索框
    let search_box = Paragraph::new(vec![
        Spans::from(format!("Search: {}", app.search_query)), // 光标占位符
    ])
    .style(Style::default().fg(Color::Yellow))
    .block(Block::default().borders(Borders::ALL).title("Search Key"));
    f.render_widget(search_box, chunks[0]);

    // 渲染过滤后的键列表
    let keys = if app.search_query.is_empty() {
        &app.keys
    } else {
        &app.search_match_keys
    };

    let items: Vec<ListItem> = keys
        .iter()
        .map(|key| {
            let style = if app.selected_key == keys.iter().position(|k| *k == *key).unwrap_or(0) {
                Style::default().bg(Color::LightBlue).fg(Color::Black)
            } else {
                Style::default()
            };
            ListItem::new(Spans::from((*key).clone())).style(style)
        })
        .collect();

    let key_list = List::new(items.clone())
        .block(Block::default().borders(Borders::ALL).title(Span::styled(
            format!("Redis Keys ({}/{})", items.len().clone(), app.keys.len()),
            Style::default().add_modifier(Modifier::BOLD),
        )))
        .highlight_style(
            Style::default()
                .bg(Color::LightBlue)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        );

    let mut state = ListState::default();
    state.select(Some(app.selected_key));
    f.render_stateful_widget(key_list, chunks[1], &mut state);
}

// 渲染键详情
fn render_key_details<B: Backend>(f: &mut Frame<B>, app: &App, area: tui::layout::Rect) {
    if let Some(key) = app.keys.get(app.selected_key) {
        if let Some(details) = app.key_details.get(key) {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(4),
                    Constraint::Min(1),
                    Constraint::Length(3),
                ])
                .split(area);

            // 键基本信息
            let details_text = vec![
                Spans::from(vec![
                    Span::styled("Key: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(key),
                ]),
                Spans::from(vec![
                    Span::styled("Type: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(&details.key_type),
                ]),
            ];

            let details_block =
                Paragraph::new(details_text).block(Block::default().borders(Borders::ALL).title(
                    Span::styled("Key Details", Style::default().add_modifier(Modifier::BOLD)),
                ));
            f.render_widget(details_block, chunks[0]);

            // 键值内容
            match details.key_type.as_str() {
                "hash" => {
                    if let Some(fields) = &details.hash_fields {
                        let mut rows = vec![Row::new(vec![
                            Cell::from(Span::styled(
                                "Field",
                                Style::default().add_modifier(Modifier::BOLD),
                            )),
                            Cell::from(Span::styled(
                                "Hash Fields",
                                Style::default().add_modifier(Modifier::BOLD),
                            )),
                        ])];

                        for (field, value) in fields {
                            rows.push(Row::new(vec![
                                Cell::from(Span::raw(field)),
                                Cell::from(Span::raw(value)),
                            ]));
                        }

                        let table = Table::new(rows)
                            // .header_style(Style::default().add_modifier(Modifier::BOLD))
                            .block(Block::default().borders(Borders::ALL).title("Hash Field"))
                            .widths(&[Constraint::Percentage(30), Constraint::Percentage(70)]);
                        f.render_widget(table, chunks[1]);
                    }
                }
                _ => {
                    let value_block = Paragraph::new(details.value.clone())
                        .block(Block::default().borders(Borders::ALL).title("Value"))
                        .wrap(tui::widgets::Wrap { trim: true });
                    f.render_widget(value_block, chunks[1]);
                }
            }

            let details_block = Paragraph::new(vec![Spans::from(vec![
                Span::styled("TTL: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(if details.ttl == -1 {
                    "Never expires".to_string()
                } else if details.ttl == -2 {
                    "Key does not exist".to_string()
                } else {
                    format!("{} seconds", details.ttl)
                }),
            ])])
            .block(Block::default().borders(Borders::ALL).title(Span::styled(
                "Key Details",
                Style::default().add_modifier(Modifier::BOLD),
            )));
            f.render_widget(details_block, chunks[2]);
        }
    }
}

fn main() -> Result<()> {
    // 解析命令行参数
    let args = Args::parse();

    // 构建Redis连接URL
    let redis_url = if let Some(url) = args.url {
        url
    } else {
        format!(
            "redis://{}:{}@{}:{}/{}",
            args.host,
            args.password.unwrap_or_default(),
            args.host,
            args.port,
            args.db
        )
    };

    // 初始化终端
    let mut terminal = init_terminal()?;
    let mut app = App::new();

    // 尝试默认连接
    if let Err(e) = app.connect_redis(&redis_url) {
        app.status = format!("Connection failed: {} URL: {}", e, redis_url);
    }

    // 主事件循环
    loop {
        // 渲染界面
        terminal.draw(|f| render(f, &app))?;

        // 处理事件
        if let Event::Key(key) = event::read()? {
            if app.handle_key_events(key.code)? {
                restore_terminal(&mut terminal)?;
                return Ok(());
            }
        }
    }
}

/// Redis TUI客户端
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Redis服务器地址
    #[arg(long, default_value = "127.0.0.1")]
    host: String,

    /// Redis服务器端口
    #[arg(long, default_value_t = 6379)]
    port: u16,

    /// Redis密码
    #[arg(long)]
    password: Option<String>,

    /// Redis数据库编号
    #[arg(short, long, default_value_t = 0)]
    db: u8,

    /// Redis连接URL (优先于单独的主机/端口参数)
    #[arg(short, long)]
    url: Option<String>,
}
