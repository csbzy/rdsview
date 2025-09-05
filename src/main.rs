mod app;
use anyhow::Result;
use app::App;

use clap::Parser;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{prelude::CrosstermBackend, Terminal};

use std::io; // Ensure these imports exist
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

fn main() -> Result<()> {
    // 解析命令行参数
    let args = Args::parse();

    // 构建Redis连接URL
    let redis_url = if let Some(url) = args.url {
        url
    } else {
        format!(
            "redis://{}:{}@{}:{}?db={}",
            args.password.as_deref().unwrap_or(""),
            args.password.as_deref().unwrap_or(""),
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
        app.set_status(format!("Connection failed: {} URL: {}", e, redis_url));
    }

    loop {
        if let Ok(true) = app.run(&mut terminal) {
            restore_terminal(&mut terminal)?;
            return Ok(());
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
