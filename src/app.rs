use anyhow::Result;
use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    prelude::CrosstermBackend,
    style::{palette::tailwind::SLATE, Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Cell, HighlightSpacing, List, ListItem, ListState, Paragraph, Row,
        ScrollDirection, Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget, Table,
        TableState, Wrap,
    },
    Frame, Terminal,
};
use redis::{Client, Commands};
use std::collections::HashMap;
use std::io; // Ensure these imports exist

const SELECTED_STYLE: Style = Style::new().bg(SLATE.c800).add_modifier(Modifier::BOLD);

// 应用状态
pub struct App {
    redis_client: Option<Client>,
    redis_connection: Option<redis::Connection>,
    keys: Vec<String>,
    search_match_keys: Vec<String>,
    key_details: HashMap<String, KeyDetails>,
    status: String,
    search_query: String,
    show_details: bool,
    key_list_state: ListState,
    key_details_vertical_scroll_state: TableState,
}

// 键详情结构
struct KeyDetails {
    key_type: String,
    ttl: i64,
    value: String,
    hash_fields: Option<HashMap<String, String>>,
}

impl App {
    pub fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<bool> {
        terminal.draw(|frame| self.render(frame))?;
        if let Event::Key(key) = event::read()? {
            return self.handle_key_events(key.code);
        }
        Ok(false)
    }
    pub fn new() -> Self {
        Self {
            redis_client: None,
            redis_connection: None,
            keys: Vec::new(),
            search_match_keys: Vec::new(),
            key_details: HashMap::new(),
            status: String::from("Not connected to Redis server"),
            search_query: String::new(),
            show_details: false,
            key_list_state: ListState::default(),
            key_details_vertical_scroll_state: TableState::default(),
        }
    }

    pub fn set_status(&mut self, status: String) {
        self.status = status;
    }

    // 连接到Redis
    pub fn connect_redis(&mut self, addr: &str) -> Result<()> {
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
            self.key_list_state.select(None);
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
                if let Some(key) = self.keys.get(self.key_list_state.selected().unwrap_or(0)) {
                    self.load_key_details(&key.clone())?;
                    self.show_details = true;
                    self.key_details_vertical_scroll_state = TableState::default();
                }
            }
            KeyCode::Esc => {
                self.show_details = false;
            }
            KeyCode::Up => {
                if self.show_details {
                    self.key_details_vertical_scroll_state.select_next();
                    return Ok(false);
                }
                if !self.keys.is_empty() {
                    if self.key_list_state.selected().is_some_and(|x| x == 0) {
                        self.key_list_state.select(Some(self.keys.len() - 1));
                    } else {
                        self.key_list_state.select_previous();
                    }
                }
            }
            KeyCode::Down => {
                if self.show_details {
                    self.key_details_vertical_scroll_state.select_previous();
                    return Ok(false);
                }
                if !self.keys.is_empty() {
                    if self
                        .key_list_state
                        .selected()
                        .is_some_and(|x| x == self.keys.len() - 1)
                    {
                        self.key_list_state.select(Some(0));
                    } else {
                        self.key_list_state.select_next();
                    }
                }
            }
            KeyCode::Char(c) => {
                if !self.show_details {
                    self.search_query.push(c);
                    self.filtered_keys();
                    self.key_list_state.select(None);
                }
            }
            KeyCode::Backspace => {
                if !self.show_details {
                    self.search_query.pop();
                    if !self.search_query.is_empty() {
                        self.filtered_keys();
                    }
                    self.key_list_state.select(None);
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

    // 渲染界面
    fn render(&mut self, frame: &mut Frame) {
        let area = frame.area();
        // 主布局
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Min(1),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .split(area);

        // 顶部状态栏
        let status_bar = Paragraph::new(self.status.clone())
            .style(Style::default().bg(Color::Blue).fg(Color::White))
            .block(Block::default().borders(Borders::NONE));
        frame.render_widget(status_bar, chunks[1]);

        if self.show_details {
            // 显示键详情
            self.render_key_details(frame, chunks[0]);
        } else {
            // 显示键列表
            self.render_key_list(frame, chunks[0]);
        }

        // 底部帮助栏
        let help_text = Line::from(vec![
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
        frame.render_widget(help_bar, chunks[2]);
    }

    // 渲染键列表
    fn render_key_list(&mut self, frame: &mut Frame, area: Rect) {
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
            Line::from(format!("Search: {}", self.search_query)), // 光标占位符
        ])
        .style(Style::default().fg(Color::Yellow))
        .block(Block::default().borders(Borders::ALL).title("Search Key"));
        frame.render_widget(search_box, chunks[0]);

        // 渲染过滤后的键列表
        let keys = if self.search_query.is_empty() {
            &self.keys
        } else {
            &self.search_match_keys
        };

        let items: Vec<ListItem> = keys
            .iter()
            .map(|key| ListItem::new(Line::from((*key).clone())))
            .collect();

        let key_list = List::new(items.clone())
            .block(Block::default().borders(Borders::ALL).title(Span::styled(
                format!("Redis Keys ({}/{})", items.len().clone(), self.keys.len()),
                Style::default().add_modifier(Modifier::BOLD),
            )))
            .highlight_style(SELECTED_STYLE)
            .highlight_symbol(">")
            .highlight_spacing(HighlightSpacing::Always)
            .scroll_padding(1);
        frame.render_stateful_widget(key_list, chunks[1], &mut self.key_list_state);
    }

    // 渲染键详情
    fn render_key_details(&mut self, frame: &mut Frame, area: Rect) {
        if let Some(key) = self.keys.get(self.key_list_state.selected().unwrap_or(0)) {
            if let Some(details) = self.key_details.get(key) {
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
                    Line::from(vec![
                        Span::styled("Key: ", Style::default().add_modifier(Modifier::BOLD)),
                        Span::raw(key),
                    ]),
                    Line::from(vec![
                        Span::styled("Type: ", Style::default().add_modifier(Modifier::BOLD)),
                        Span::raw(&details.key_type),
                    ]),
                ];

                let details_block = Paragraph::new(details_text).block(
                    Block::default().borders(Borders::ALL).title(Span::styled(
                        "Key Details",
                        Style::default().add_modifier(Modifier::BOLD),
                    )),
                );
                frame.render_widget(details_block, chunks[0]);

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

                            let widths = [Constraint::Length(5), Constraint::Length(5)];
                            // 更新滚动状态
                            self.key_details_vertical_scroll_state.select_first();

                            let table = Table::new(rows, widths)
                                .block(Block::default().borders(Borders::ALL).title("Hash Field"))
                                .widths(&[Constraint::Percentage(30), Constraint::Percentage(70)])
                                .cell_highlight_style(SELECTED_STYLE)
                                .column_highlight_style(SELECTED_STYLE);

                            frame.render_stateful_widget(
                                table,
                                chunks[1],
                                &mut self.key_details_vertical_scroll_state,
                            );
                        }
                    }
                    _ => {
                        let value_block = Paragraph::new(details.value.clone())
                            .block(Block::default().borders(Borders::ALL).title("Value"))
                            .wrap(Wrap { trim: true });
                        frame.render_widget(value_block, chunks[1]);
                    }
                }

                let details_block = Paragraph::new(vec![Line::from(vec![
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
                frame.render_widget(details_block, chunks[2]);
            }
        }
    }
}
